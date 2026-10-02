#!/usr/bin/env python3
"""Patch Godot's iOS export template with WRY's required system frameworks.

Godot WRY is shipped as a static XCFramework. Cargo's native link metadata is
not embedded in a static archive, so the generated Xcode project must link the
system WebKit framework explicitly.
"""

from __future__ import annotations

import argparse
import tempfile
import zipfile
from pathlib import Path

NEEDLE = 'OTHER_LDFLAGS = "$(LD_CLASSIC_$(XCODE_VERSION_ACTUAL)) $linker_flags";'
REPLACEMENT = (
    'OTHER_LDFLAGS = "$(LD_CLASSIC_$(XCODE_VERSION_ACTUAL)) '
    '$linker_flags -framework WebKit";'
)


def patch_template(path: Path) -> None:
    path = path.resolve()
    with zipfile.ZipFile(path, "r") as source:
        infos = source.infolist()
        payloads = {info.filename: source.read(info.filename) for info in infos}

    candidates = [
        name for name in payloads
        if name.endswith(".xcodeproj/project.pbxproj")
    ]
    if len(candidates) != 1:
        raise RuntimeError(
            f"Expected exactly one Xcode project template in {path}, got {candidates}"
        )

    project_name = candidates[0]
    project = payloads[project_name].decode("utf-8")
    if REPLACEMENT in project:
        print(f"WebKit linker flag already present in {path}")
        return

    occurrences = project.count(NEEDLE)
    if occurrences != 2:
        raise RuntimeError(
            f"Expected Debug + Release linker settings in {project_name}; "
            f"found {occurrences}"
        )
    payloads[project_name] = project.replace(NEEDLE, REPLACEMENT).encode("utf-8")

    with tempfile.NamedTemporaryFile(
        prefix=path.stem + ".", suffix=".zip", dir=path.parent, delete=False
    ) as temp:
        temp_path = Path(temp.name)

    try:
        with zipfile.ZipFile(temp_path, "w") as output:
            for info in infos:
                output.writestr(info, payloads[info.filename])
        temp_path.replace(path)
    finally:
        temp_path.unlink(missing_ok=True)

    with zipfile.ZipFile(path, "r") as verify:
        patched = verify.read(project_name).decode("utf-8")
        if REPLACEMENT not in patched:
            raise RuntimeError("Patched template verification failed")
        bad = verify.testzip()
        if bad is not None:
            raise RuntimeError(f"ZIP CRC validation failed for {bad}")

    print(f"Patched WRY WebKit linker flag into {path}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("template", type=Path)
    args = parser.parse_args()
    patch_template(args.template)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
