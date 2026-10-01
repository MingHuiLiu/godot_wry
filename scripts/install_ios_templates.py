#!/usr/bin/env python3
"""Install the Godot 4.7 iOS export templates bundled with a godot_wry release.

This script performs no network access. It copies the Standard and Mono iOS
templates shipped in the release into Godot's per-version export-template
folders on macOS.
"""

from __future__ import annotations

import argparse
from pathlib import Path
import shutil
import sys

TEMPLATES = {
    "standard": "4.7.stable",
    "mono": "4.7.stable.mono",
}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--edition",
        choices=("all", "standard", "mono"),
        default="all",
        help="Template edition to install (default: all).",
    )
    parser.add_argument(
        "--godot-data-dir",
        type=Path,
        default=Path.home() / "Library" / "Application Support" / "Godot",
        help="Godot user data directory.",
    )
    args = parser.parse_args()

    if sys.platform != "darwin":
        print("[godot_wry] iOS export templates can only be installed on macOS.", file=sys.stderr)
        return 2

    script_dir = Path(__file__).resolve().parent
    template_root = script_dir / "templates"

    editions = TEMPLATES.keys() if args.edition == "all" else (args.edition,)
    installed = []

    for edition in editions:
        template_id = TEMPLATES[edition]
        source = template_root / template_id / "ios.zip"
        if not source.is_file():
            print(f"[godot_wry] Bundled template is missing: {source}", file=sys.stderr)
            return 2

        destination_dir = args.godot_data_dir / "export_templates" / template_id
        destination_dir.mkdir(parents=True, exist_ok=True)
        destination = destination_dir / "ios.zip"
        shutil.copy2(source, destination)
        installed.append(destination)
        print(f"[godot_wry] Installed {edition} iOS template: {destination}")

    print("[godot_wry] iOS templates are ready. Export directly from Godot 4.7.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())