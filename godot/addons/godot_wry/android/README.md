# Android integration

Godot WRY 0.3.0 publishes two **turnkey Android** Release packages:

- `godot_wry-android-godot4.7-standard.zip`
- `godot_wry-android-godot4.7-mono.zip`

Choose the package that matches the project's Godot 4.7 edition and extract it
into the **project root**.

Each turnkey package already contains:

- `addons/godot_wry/bin/android/arm64-v8a/libgodot_wry.so`
- `addons/godot_wry/bin/android/x86_64/libgodot_wry.so`
- WRY-generated Kotlin/JNI bridge sources
- a WRY-compatible `WryActivity` that subclasses Godot's `GodotActivity`
- the matching official Godot 4.7 `android/build` custom Gradle template
- matching official Godot debug/release AARs
- Gradle wrapper files
- the correct Godot `android/.build_version` marker
- `godot_wry.gradle` already applied to the custom build

With a turnkey ZIP, **do not install another Android Build Template and do not
run `install.py`**. Enable **Gradle Build** in the Android export preset and
export normally.

During export, the packaged Gradle integration changes Godot's generated
activity references to `com.example.godotwry.WryActivity` and adds the
AndroidX WebKit dependency required by WRY.

## Advanced: existing custom Android Build Template

If a project intentionally owns and maintains its own `android/build` tree,
the standalone installer is still available:

```sh
python addons/godot_wry/android/install.py
```

The installer is idempotent and only applies the WRY Gradle integration. It is
not needed for the turnkey packages.

Android support targets Godot 4.7 and ships arm64-v8a (devices) and x86_64
(emulators).
