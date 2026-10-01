# iOS turnkey package

The GitHub iOS turnkey Release package bundles both official Godot 4.7
`ios.zip` export templates alongside Godot WRY:

- Standard
- Mono/.NET

After extracting the package into a Godot project, install the bundled template
matching your editor once (no network download):

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
