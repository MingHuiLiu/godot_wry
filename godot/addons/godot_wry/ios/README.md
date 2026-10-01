# iOS turnkey package

The edition-specific GitHub Release packages bundle Godot 4.7's official
`ios.zip` export template alongside Godot WRY.

After extracting the package into a Godot project, install the bundled template
once (no network download):

```bash
python addons/godot_wry/ios/install.py --edition standard
# or:
python addons/godot_wry/ios/install.py --edition mono
```

The WRY GDExtension itself is a static XCFramework containing:
- iOS device arm64
- iOS Simulator arm64 + x86_64

Godot 4.7's GDExtension exporter links static Apple Embedded XCFrameworks into
the generated Xcode project and registers the GDExtension entry symbol.
