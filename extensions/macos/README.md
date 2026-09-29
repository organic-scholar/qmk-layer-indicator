# QMK Layer Indicator macOS menu bar app

Shows the layer received from the QMK Layer Indicator WebSocket server in the
macOS menu bar. This is the macOS equivalent of the GNOME Shell top-panel
extension: the menu bar item shows the layer icon followed by the configured
alias (falling back to the numeric layer), and it does not access the keyboard
HID device itself — the Rust backend service does that and publishes layers over
the local WebSocket.

The app is an AppKit `NSStatusItem` accessory app (`LSUIElement`), so it has no
Dock icon or main window; it only adds a single menu bar item. The layer glyph
is rendered as a template image, so the menu bar tints it automatically for
light and dark appearances. It requires macOS 13 or later.

## Build and install

Start the QMK Layer Indicator backend first (see the repository README /
`just install-macos`), then build and install this menu bar app:

```sh
just install-macos-menubar
```

That builds the Swift package, bundles it into
`~/Applications/QMK Layer Indicator Menu Bar.app`, and installs a per-user
LaunchAgent (`com.organic-scholar.qmk-layer-indicator.menubar`) so it starts at
login. To only build a distributable `.app` under
`packaging/macos/dist/` without installing it:

```sh
just macos-menubar-package
```

To remove the login agent:

```sh
just macos-menubar-remove-login-agent
```

## Building directly with Swift

```sh
cd extensions/macos
swift build -c release
```

The executable is produced at `.build/release/QmkLayerIndicatorMenuBar`, but it
must be run from an `.app` bundle (with `LSUIElement`) to behave as a menu bar
accessory; use the `just` recipes above to produce one.

## Configuration

The app uses `$QMK_LAYER_INDICATOR_WEBSOCKET_URL` when set; otherwise it connects
to the application default of `ws://127.0.0.1:51837`. Events are JSON such as
`{"layer":1,"alias":"S"}`; an unconfigured alias is an empty string, in which
case the numeric layer is shown. While the backend is unavailable the menu bar
shows `?` and the app retries every 3 seconds.
