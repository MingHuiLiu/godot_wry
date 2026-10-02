#!/usr/bin/env python3
"""Download one member from a remote ZIP using HTTP Range requests.

This is used by CI to extract Godot's platform-specific export-template ZIPs
from the large .tpz archives without downloading the entire archive.
"""

from __future__ import annotations

import argparse
import binascii
import struct
import sys
import time
import urllib.error
import urllib.request
import zlib
from pathlib import Path

EOCD = b"PK\x05\x06"
CENTRAL = b"PK\x01\x02"
LOCAL = b"PK\x03\x04"
USER_AGENT = "godot-wry-ci/0.3"


def get_range(url: str, start: int, end: int) -> tuple[bytes, dict[str, str], int]:
    last_error: Exception | None = None
    for attempt in range(5):
        req = urllib.request.Request(
            url,
            headers={
                "Range": f"bytes={start}-{end}",
                "User-Agent": USER_AGENT,
                "Accept": "*/*",
            },
        )
        try:
            with urllib.request.urlopen(req, timeout=120) as response:
                data = response.read()
                headers = {k.lower(): v for k, v in response.headers.items()}
                return data, headers, response.status
        except urllib.error.HTTPError as exc:
            last_error = exc
            if exc.code not in {429, 500, 502, 503, 504}:
                raise
        except (urllib.error.URLError, TimeoutError) as exc:
            last_error = exc

        if attempt < 4:
            delay = 2 ** attempt
            print(
                f"Range {start}-{end} failed ({last_error}); retrying in {delay}s...",
                file=sys.stderr,
            )
            time.sleep(delay)

    assert last_error is not None
    raise last_error


def get_range_chunked(
    url: str, start: int, end: int, chunk_size: int = 8 * 1024 * 1024
) -> bytes:
    chunks: list[bytes] = []
    current = start
    while current <= end:
        chunk_end = min(end, current + chunk_size - 1)
        data, _, status = get_range(url, current, chunk_end)
        if status != 206:
            raise RuntimeError(
                f"Server did not honor Range request {current}-{chunk_end} "
                f"(status={status})"
            )
        expected = chunk_end - current + 1
        if len(data) != expected:
            raise RuntimeError(
                f"Range length mismatch for {current}-{chunk_end}: "
                f"expected {expected}, got {len(data)}"
            )
        chunks.append(data)
        current = chunk_end + 1
    return b"".join(chunks)


def remote_size(url: str) -> int:
    data, headers, status = get_range(url, 0, 0)
    if status != 206:
        raise RuntimeError(f"Server did not honor Range request (status={status})")
    content_range = headers.get("content-range", "")
    if "/" not in content_range:
        raise RuntimeError(f"Missing Content-Range header: {content_range!r}")
    size = int(content_range.rsplit("/", 1)[1])
    if len(data) != 1:
        raise RuntimeError(f"Expected one ranged byte, got {len(data)}")
    return size


def find_member(url: str, member: str) -> tuple[int, int, int, int, int]:
    size = remote_size(url)
    tail_size = min(size, 131072)
    tail_start = size - tail_size
    tail, _, status = get_range(url, tail_start, size - 1)
    if status != 206:
        raise RuntimeError("Could not read ZIP tail")

    eocd_at = tail.rfind(EOCD)
    if eocd_at < 0:
        raise RuntimeError("ZIP end-of-central-directory record not found")

    eocd = tail[eocd_at : eocd_at + 22]
    (
        signature,
        _disk_no,
        _cd_disk,
        _entries_disk,
        entries_total,
        cd_size,
        cd_offset,
        _comment_len,
    ) = struct.unpack("<4s4H2IH", eocd)
    if signature != EOCD:
        raise RuntimeError("Invalid EOCD signature")

    cd_end = cd_offset + cd_size - 1
    central = get_range_chunked(url, cd_offset, cd_end)

    pos = 0
    for _ in range(entries_total):
        if central[pos : pos + 4] != CENTRAL:
            raise RuntimeError(f"Invalid central directory entry at offset {pos}")

        method = struct.unpack_from("<H", central, pos + 10)[0]
        crc32 = struct.unpack_from("<I", central, pos + 16)[0]
        compressed_size = struct.unpack_from("<I", central, pos + 20)[0]
        uncompressed_size = struct.unpack_from("<I", central, pos + 24)[0]
        name_len = struct.unpack_from("<H", central, pos + 28)[0]
        extra_len = struct.unpack_from("<H", central, pos + 30)[0]
        comment_len = struct.unpack_from("<H", central, pos + 32)[0]
        local_offset = struct.unpack_from("<I", central, pos + 42)[0]

        name_start = pos + 46
        name = central[name_start : name_start + name_len].decode("utf-8")
        if name == member:
            return local_offset, compressed_size, uncompressed_size, method, crc32

        pos += 46 + name_len + extra_len + comment_len

    raise FileNotFoundError(f"{member!r} not found in remote ZIP")


def extract_member(url: str, member: str, output: Path) -> None:
    local_offset, compressed_size, uncompressed_size, method, expected_crc = find_member(
        url, member
    )

    header, _, status = get_range(url, local_offset, local_offset + 29)
    if status != 206 or header[:4] != LOCAL:
        raise RuntimeError("Invalid local ZIP header")

    name_len = struct.unpack_from("<H", header, 26)[0]
    extra_len = struct.unpack_from("<H", header, 28)[0]
    data_start = local_offset + 30 + name_len + extra_len
    compressed = get_range_chunked(
        url, data_start, data_start + compressed_size - 1
    )

    if method == 0:
        payload = compressed
    elif method == 8:
        payload = zlib.decompress(compressed, -zlib.MAX_WBITS)
    else:
        raise RuntimeError(f"Unsupported ZIP compression method: {method}")

    if len(payload) != uncompressed_size:
        raise RuntimeError(
            f"Uncompressed size mismatch: expected {uncompressed_size}, got {len(payload)}"
        )

    actual_crc = binascii.crc32(payload) & 0xFFFFFFFF
    if actual_crc != expected_crc:
        raise RuntimeError(
            f"CRC mismatch: expected {expected_crc:08x}, got {actual_crc:08x}"
        )

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(payload)
    print(
        f"Extracted {member} -> {output} "
        f"({len(payload) / 1024 / 1024:.1f} MiB)"
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("url")
    parser.add_argument("member")
    parser.add_argument("output", type=Path)
    args = parser.parse_args()

    try:
        extract_member(args.url, args.member, args.output)
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
