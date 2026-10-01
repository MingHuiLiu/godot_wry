<a href="https://godotengine.org/asset-library/asset/3426">
  <img src="assets/splash.png" />
</a>

<p align="center">
  <img src="https://img.shields.io/static/v1?label=Godot&message=4.7%2B&color=478CBF&logo=godotengine">
  <img src="https://github.com/MingHuiLiu/godot_wry/actions/workflows/build.yml/badge.svg">
  <a href="https://discord.gg/B9fWw3raZJ">
    <img src="https://img.shields.io/static/v1?label=Need%20help%3F&message=Join%20us%20on%20Discord!&color=5865F2&logo=discord">
  </a>
</p>

# Godot WRY

[WRY](https://github.com/tauri-apps/wry) is a cross-platform webview rendering library. This extension allows you to use the native webview in Godot to build browsers and GUIs with HTML, CSS and JavaScript.

## ✨ Features

- 🍃 Use the native system native webview (no extra dependencies!)
- 🌎 Load website URLs and local `res://` files
- 🧩 JavaScript ⇔ GDScript code integration
- 🚥 Mouse/keyboard input events forwarding
- 🧩 Runs inside Godot 4.7's embedded **Game** workspace, including macOS

## ⛹️ Demo

<p align="center">
  <img src="assets/demo-cas.gif">
  Demo game UI available at "<a href="godot/addons/godot_wry/examples/character_creator_ui_demo">examples/character_creator_ui_demo</a>".
</p>

<details>
  <summary>📸 Other screenshots</summary>
  
  ![](assets/screenshot-7.png)
  ![](assets/screenshot-6.png)
  ![](assets/screenshot-4.png)
  ![](assets/screenshot-5.png)
  
</details>

## 💾 Installing

### Asset Library

The easiest way to install Godot WRY is through Godot's [Asset Library](https://godotengine.org/asset-library/asset/3426). You can install it via the editor by following these instructions:

1. Open your project in Godot 4.7 or later.
2. Go to the "📥 AssetLib" tab at the top of the editor.
3. Search for "Godot WRY".
4. Click on the Godot WRY extension and click **Download**.
5. In the configuration dialog, click **Install**.

### GitHub Releases

Starting with **v0.3.0**, mobile packages are self-contained for Godot 4.7:

- `godot_wry.zip` — all-platform addon binaries.
- `godot_wry-android-godot4.7.zip` — Android package for Godot 4.7 Standard.
- `godot_wry-android-godot4.7-mono.zip` — Android package for Godot 4.7 Mono.
- `godot_wry-ios-godot4.7.zip` — iOS package with device/simulator XCFramework and bundled Standard + Mono Godot 4.7 iOS export templates.

#### Android

Choose the package matching your Godot editor edition and extract it into the **project root**. It already contains `addons/godot_wry` and a complete, pre-integrated `android/build` Gradle Build Template with the official Godot 4.7 AARs, WRY Kotlin bridge and `WryActivity`.

Enable **Gradle Build** in the Android export preset and export normally. No extra template download and no `install.py` step are required for these Release bundles.

#### iOS

Extract `godot_wry-ios-godot4.7.zip` into the project root. The addon includes:

- arm64 iPhone/iPad device support.
- arm64 + x86_64 iOS Simulator support.
- the official Godot 4.7 Standard and Mono `ios.zip` export templates.

Install the already-bundled templates once on the Mac:

```sh
python3 godot_wry_ios/install_templates.py
```

The installer only copies files from the Release into Godot's local export-template directories; it performs **no network download**. After that, export iOS normally from Godot 4.7.

### Godot 4.7 embedded Game workspace

Godot WRY 0.2.0 supports running the game inside the editor's **Game** workspace.

- **Windows / Linux:** WRY attaches to the native game child window that Godot embeds in the editor.
- **macOS:** Godot 4.7 renders embedded games through a cross-process `CAContext/CALayer`, so the game process no longer owns an `NSView` that can host `WKWebView`. Godot WRY detects `Engine.is_embedded_in_editor()` and automatically mirrors WebView state to the editor process over a project-local Unix socket. The editor-hosted WKWebView is positioned over the visible `GamePanel`.
- WebView IPC and page-load signals are forwarded back to the running game, and forwarded mouse/keyboard coordinates are remapped when the Game workspace scales the game.
- Moving the Game workspace between the main editor window and a floating window is handled by native WRY reparenting.

No additional EditorPlugin needs to be enabled.

### Build from source

Use [just](https://github.com/casey/just) to build the extension and move the binaries to the Godot project folder:

```sh
$ just build
```

If you need a more in-depth guide on how to compile the project, check the [Building from source](https://godot-wry.doce.sh/contributing/compiling.html) documentation page.

## 📚 Documentation

Please refer to the [Docs](https://godot-wry.doce.sh) for API reference and in-depth guides on how to use Godot WRY.

## 🎯 Supported platforms

| Platform                        | Support        | Web engine                 |
| ------------------------------- | -------------- | -------------------------- |
| **Windows (10, 11)**            | ✅ Supported   | WebView2 (Chromium)        |
| **Mac (Intel, Apple Sillicon)** | ✅ Supported   | WebKit                     |
| **Linux (X11)**                 | 🚧 Supported\* | WebKitGTK                  |
| **Android (arm64, x86_64)**     | ✅ Supported   | Android WebView (Chromium) |
| **iOS (device + simulator)**     | ✅ Supported   | WebKit                     |
| **Browser/HTML5**               | ⏳ Planned     | —                          |

### Linux

[WebKitGTK](https://webkitgtk.org) is required for WRY to function on Linux. The package name may differ based on the operating system and Linux distribution.

\* X11 support only. Transparency is currently not supported. See [#17](https://github.com/doceazedo/godot_wry/issues/17).

### Android

Release bundles include the complete Godot 4.7 Gradle Build Template and are provided separately for Standard and Mono so the embedded Godot runtime exactly matches the editor edition. Native WRY libraries are built for arm64-v8a devices and x86_64 emulators.

The lower-level `addons/godot_wry/android/install.py` remains available for users maintaining a custom Android Build Template, but it is not needed for the self-contained v0.3.0 Release bundles.

### iOS

The iOS Release includes a single XCFramework with an arm64 device slice and a universal arm64/x86_64 simulator slice. Because Godot stores iOS export templates in its user-level template directory rather than inside each project, the Release includes the official Godot 4.7 Standard and Mono `ios.zip` files plus a local copy-only installer.

## ❌ Caveats

- Godot **4.7 or newer** is required by this fork/release.
- Webview always renders on top
- Different browser engines across platforms
- No automatic dependency checks

You can learn more about these caveats on the [Caveats](https://godot-wry.doce.sh/about/caveats.html) documentation page.

## 🤝 Contribute

Your help is most welcome regardless of form! Check out the [How to contribute](https://godot-wry.doce.sh/contributing/how-to-contribute.html) page for all ways you can contribute to the project. For example, [suggest a new feature](https://github.com/doceazedo/godot_wry/issues/new?template=feature_request.md), [report a problem/bug](https://github.com/doceazedo/godot_wry/issues/new?template=bug_report.md), [submit a pull request](https://help.github.com/en/github/collaborating-with-issues-and-pull-requests/about-pull-requests), or simply use the project and comment your experience.

See the [Roadmap](https://godot-wry.doce.sh/about/roadmap.html) documentation page for an idea of how the project should evolve.

## 🎫 License

The Godot WRY extension is licensed under [MIT](/LICENSE). WRY is licensed under [Apache-2.0/MIT](https://github.com/tauri-apps/wry/blob/dev/LICENSE.spdx).

## 🧪 Similar projects

Below is a list of interesting similar projects:

- [gdcef](https://github.com/Lecrapouille/gdcef/tree/godot-4.x) — Open-source, powered by Chromium (CEF)
- [Godot-HTML](https://github.com/Decapitated/Godot-HTML) — Open-source, powered by Ultralight (WebKit)
- [godot-webview](https://godotwebview.com/) — Commercial, powered by Qt6 (Chromium)
