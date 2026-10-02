# iOS turnkey package

The GitHub iOS turnkey Release package bundles both official Godot 4.7
`ios.zip` export templates alongside Godot WRY:

- Standard
- Mono/.NET

After extracting the package into a Godot project, simply open the project with
Godot 4.7 on macOS. The GDExtension automatically installs both bundled iOS
templates into Godot's local export-template directories, entirely offline.

If an existing `ios.zip` differs, Godot WRY keeps a one-time
`ios.zip.godot-wry-original` backup before installing its WebKit-linked
template.

`install.py` is retained as a manual fallback:

```bash
python addons/godot_wry/ios/install.py
```

The WRY GDExtension itself is a static XCFramework containing:
- iOS device arm64
- iOS Simulator arm64 + x86_64

Godot 4.7's GDExtension exporter links static Apple Embedded XCFrameworks into
the generated Xcode project and registers the GDExtension entry symbol.
