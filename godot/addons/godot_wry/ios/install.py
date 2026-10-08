#!/usr/bin/env python3
"""Install the bundled Godot 4.7 iOS export template without downloading anything.

The turnkey iOS Release packages include the official Godot ios.zip under
addons/godot_wry/ios/templates/ios.zip. This script copies that file into the
Godot export-template directory for the matching Standard or Mono edition.
"""

from __future__ import annotations

import argparse
import filecmp
import platform
import shutil
from pathlib import Path
import sys

GODOT_VERSION = "4.7.stable"


def default_template_root() -> Path:
    system = platform.system()
    home = Path.home()
    if system == "Darwin":
        return home / "Library" / "Application Support" / "Godot" / "export_templates"
    if system == "Windows":
        appdata = Path.home() / "AppData" / "Roaming"
        return appdata / "Godot" / "export_templates"
    return home / ".local" / "share" / "godot" / "export_templates"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--edition",
        choices=("standard", "mono", "all"),
        default="all",
        help=(
            "Install one Godot edition, or both (default: all). "
            "Installing both lets Standard and Mono editors export without downloads."
        ),
    )
    parser.add_argument(
        "--template-root",
        type=Path,
        default=default_template_root(),
        help="Override Godot's export_templates directory.",
    )
    args = parser.parse_args()

    script_dir = Path(__file__).resolve().parent
    editions = ("standard", "mono") if args.edition == "all" else (args.edition,)
    template_root = args.template_root.expanduser().resolve()

    for edition in editions:
        bundled = script_dir / "templates" / edition / "ios.zip"
        if not bundled.is_file():
            print(
                f"[godot_wry] Bundled {edition} ios.zip is missing. "
                "Use the Godot 4.7 turnkey iOS Release package.",
                file=sys.stderr,
            )
            return 2

        version_dir = GODOT_VERSION + (".mono" if edition == "mono" else "")
        destination_dir = template_root / version_dir
        destination = destination_dir / "ios.zip"
        destination_dir.mkdir(parents=True, exist_ok=True)

        if destination.is_file() and filecmp.cmp(bundled, destination, shallow=False):
            print(f"[godot_wry] {edition} iOS template is already current: {destination}")
            continue

        if destination.is_file():
            backup = destination_dir / "ios.zip.godot-wry-original"
            if not backup.exists():
                shutil.copy2(destination, backup)
                print(f"[godot_wry] Backed up existing template: {backup}")

        shutil.copy2(bundled, destination)
        print(f"[godot_wry] Installed {edition} iOS template: {destination}")

    print("[godot_wry] No network download was required.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
