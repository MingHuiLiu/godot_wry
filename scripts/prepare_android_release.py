#!/usr/bin/env python3
"""Build a zero-download Android project bundle from Godot's official template.

The input android_source.zip is copied verbatim into <package>/android/build,
then the generated godot_wry Kotlin/Gradle layer is wired in. The resulting
package can be extracted directly over a Godot 4.7 project and exported with
Gradle Build enabled; no "Install Android Build Template" step is required.
"""

from __future__ import annotations

import argparse
from pathlib import Path
import shutil
import stat
import subprocess
import sys
import zipfile


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--package-root", type=Path, required=True)
    parser.add_argument("--android-source", type=Path, required=True)
    parser.add_argument("--template-id", required=True)
    args = parser.parse_args()

    package_root = args.package_root.resolve()
    source_zip = args.android_source.resolve()
    android_dir = package_root / "android"
    build_dir = android_dir / "build"

    if not (package_root / "project.godot").exists():
        # Release packages are overlays, not standalone projects. A temporary
        # project.godot lets the existing installer perform its safety checks.
        temporary_project = True
        (package_root / "project.godot").write_text(
            '[application]\nconfig/name="godot_wry Android release overlay"\n',
            encoding="utf-8",
        )
    else:
        temporary_project = False

    if not source_zip.is_file():
        print(f"[godot_wry] Missing Android source template: {source_zip}", file=sys.stderr)
        return 2

    shutil.rmtree(build_dir, ignore_errors=True)
    build_dir.mkdir(parents=True, exist_ok=True)

    with zipfile.ZipFile(source_zip) as archive:
        archive.extractall(build_dir)

    # Godot's project scanner must never recurse into the Gradle project.
    android_dir.mkdir(parents=True, exist_ok=True)
    (android_dir / ".gdignore").write_text("\n", encoding="utf-8")
    (build_dir / ".gdignore").write_text("\n", encoding="utf-8")
    (android_dir / ".build_version").write_text(args.template_id + "\n", encoding="utf-8")

    gradlew = build_dir / "gradlew"
    if gradlew.exists():
        gradlew.chmod(gradlew.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)

    installer = package_root / "addons" / "godot_wry" / "android" / "install.py"
    if not installer.is_file():
        print(f"[godot_wry] Missing Android installer: {installer}", file=sys.stderr)
        return 2

    subprocess.run([sys.executable, str(installer), str(package_root)], check=True)

    if temporary_project:
        (package_root / "project.godot").unlink(missing_ok=True)

    required = [
        build_dir / "build.gradle",
        build_dir / "config.gradle",
        build_dir / "gradlew",
        package_root / "addons" / "godot_wry" / "android" / "kotlin" / "com" / "example" / "godotwry" / "WryActivity.kt",
    ]
    missing = [str(path) for path in required if not path.exists()]
    if missing:
        print("[godot_wry] Android release bundle is incomplete:\n  " + "\n  ".join(missing), file=sys.stderr)
        return 2

    print(f"[godot_wry] Prepared Android {args.template_id} bundle at {package_root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())