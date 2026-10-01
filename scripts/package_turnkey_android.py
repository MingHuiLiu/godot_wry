#!/usr/bin/env python3
"""Assemble a project-root Android turnkey package for Godot WRY.

The input android_source.zip is the exact Godot custom Android build template
from the matching official export-template archive.
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path


def copy_addon(repo: Path, output: Path, addon_source: Path | None) -> None:
    source = addon_source if addon_source is not None else repo / "godot" / "addons" / "godot_wry"
    target = output / "addons" / "godot_wry"
    if target.exists():
        shutil.rmtree(target)
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(source, target)


def extract_android_template(source_zip: Path, output: Path, template_id: str) -> None:
    android_root = output / "android"
    build_root = android_root / "build"
    if android_root.exists():
        shutil.rmtree(android_root)
    build_root.mkdir(parents=True, exist_ok=True)

    with zipfile.ZipFile(source_zip) as archive:
        archive.extractall(build_root)

    gradlew = build_root / "gradlew"
    if gradlew.exists():
        gradlew.chmod(0o755)

    # Godot checks this marker before accepting an existing custom build.
    (android_root / ".build_version").write_text(template_id + "\n", encoding="utf-8")

    # Keep the editor file scanner out of Gradle's generated resource tree.
    (android_root / ".gdignore").write_text("\n", encoding="utf-8")
    (build_root / ".gdignore").write_text("\n", encoding="utf-8")


def install_wry(output: Path) -> None:
    # The installer intentionally checks for project.godot. Add a temporary one
    # while staging the release; end-user projects already have their own.
    project_file = output / "project.godot"
    project_file.write_text("[application]\nconfig/name=\"Godot WRY turnkey staging\"\n", encoding="utf-8")
    try:
        installer = output / "addons" / "godot_wry" / "android" / "install.py"
        subprocess.run([sys.executable, str(installer), str(output)], check=True)
    finally:
        project_file.unlink(missing_ok=True)


def validate(output: Path) -> None:
    required = [
        output / "addons" / "godot_wry" / "WRY.gdextension",
        output / "addons" / "godot_wry" / "bin" / "android" / "arm64-v8a" / "libgodot_wry.so",
        output / "addons" / "godot_wry" / "bin" / "android" / "x86_64" / "libgodot_wry.so",
        output / "android" / "build" / "build.gradle",
        output / "android" / "build" / "gradlew",
        output / "android" / ".build_version",
    ]
    missing = [str(path) for path in required if not path.exists()]
    if missing:
        raise RuntimeError("Missing turnkey files:\n" + "\n".join(missing))

    gradle = (output / "android" / "build" / "build.gradle").read_text(encoding="utf-8")
    marker = 'apply from: "../../addons/godot_wry/android/godot_wry.gradle"'
    if marker not in gradle:
        raise RuntimeError("WRY Gradle integration was not installed")

    debug_aars = list((output / "android" / "build" / "libs" / "debug").glob("*.aar"))
    release_aars = list((output / "android" / "build" / "libs" / "release").glob("*.aar"))
    if not debug_aars or not release_aars:
        raise RuntimeError("Official Godot debug/release AARs are missing from the template")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, default=Path("."))
    parser.add_argument("--android-source", type=Path, required=True)
    parser.add_argument(
        "--addon-source",
        type=Path,
        help="Preassembled addon directory (used to retain desktop host binaries)",
    )
    parser.add_argument("--template-id", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    repo = args.repo.resolve()
    output = args.output.resolve()
    if output.exists():
        shutil.rmtree(output)
    output.mkdir(parents=True)

    addon_source = args.addon_source.resolve() if args.addon_source else None
    copy_addon(repo, output, addon_source)
    extract_android_template(args.android_source.resolve(), output, args.template_id)
    install_wry(output)
    validate(output)

    print(f"Turnkey Android package ready: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
