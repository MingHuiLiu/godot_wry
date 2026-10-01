# Android integration

The packaged Android artifact contains:

- `addons/godot_wry/bin/android/arm64-v8a/libgodot_wry.so`
- `addons/godot_wry/bin/android/x86_64/libgodot_wry.so`
- WRY-generated Kotlin bridge sources under `addons/godot_wry/android/kotlin/`
- a WRY-compatible `WryActivity` that subclasses Godot's `GodotActivity`
- `godot_wry.gradle`, which wires the bridge into Godot's custom Android build

## Existing Godot project

1. Copy/extract the artifact into the project root so `addons/godot_wry` exists.
2. In Godot, run **Project > Install Android Build Template** if the project does not already contain `android/build`.
3. Run:

   ```sh
   python addons/godot_wry/android/install.py
   ```

4. Export Android with **Gradle Build** enabled.

The installer is idempotent. It only adds the addon Gradle integration to the
Godot Android build template. During each export, that integration patches
Godot's generated activity references to `com.example.godotwry.WryActivity`
and adds the AndroidX WebKit dependency required by WRY.

Android support targets Godot 4.2+ and currently ships arm64-v8a (devices) and
x86_64 (emulators).
