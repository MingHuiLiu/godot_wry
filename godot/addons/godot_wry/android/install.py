#!/usr/bin/env python3
"""Install godot_wry's Android Gradle integration into an existing Godot project.

Run this from the project root after Godot's "Install Android Build Template".
The operation is idempotent and only appends one apply-from line.
"""

from __future__ import annotations

import argparse
from pathlib import Path
import sys

MARKER = 'apply from: "../../addons/godot_wry/android/godot_wry.gradle"'


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "project",
        nargs="?",
        default=".",
        help="Godot project root (default: current directory)",
    )
    args = parser.parse_args()

    project = Path(args.project).resolve()
    project_file = project / "project.godot"
    gradle_file = project / "android" / "build" / "build.gradle"
    integration = project / "addons" / "godot_wry" / "android" / "godot_wry.gradle"
    kotlin_dir = project / "addons" / "godot_wry" / "android" / "kotlin"

    if not project_file.exists():
        print(f"[godot_wry] project.godot not found: {project}", file=sys.stderr)
        return 2
    if not integration.exists():
        print("[godot_wry] Android integration files are missing from the addon.", file=sys.stderr)
        return 2
    if not kotlin_dir.exists():
        print(
            "[godot_wry] Generated WRY Kotlin bridge is missing. "
            "Use the packaged Android/Godot artifact produced by GitHub Actions.",
            file=sys.stderr,
        )
        return 2
    if not gradle_file.exists():
        print(
            "[godot_wry] android/build/build.gradle not found. "
            "In Godot choose Project > Install Android Build Template, then run this command again.",
            file=sys.stderr,
        )
        return 2

    text = gradle_file.read_text(encoding="utf-8")
    if MARKER not in text:
        if not text.endswith("\n"):
            text += "\n"
        text += "\n// godot_wry Android integration\n" + MARKER + "\n"
        gradle_file.write_text(text, encoding="utf-8")
        print(f"[godot_wry] Patched {gradle_file}")
    else:
        print("[godot_wry] Android Gradle integration is already installed.")

    print("[godot_wry] Android template is ready. Export with Gradle Build enabled.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
