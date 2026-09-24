# QMK Layer Indicator for VS Code

This extension connects to the QMK Layer Indicator application and updates the
in-memory cursor style of visible VS Code editors when it receives a layer event.
It does not modify `settings.json` or the user's `editor.cursorStyle` preference.
It also shows the current QMK layer in VS Code's status bar.

By default, layer `0` selects the line cursor and layer `1` selects the block
cursor. Configure additional layers in VS Code settings:

```json
{
  "qmkLayerIndicator.layerCursorStyles": {
    "0": "line",
    "1": "block",
    "2": "underline"
  }
}
```


## Setup

Start the application normally. Its layer socket is always available:

```sh
qmk-layer-indicator
```

To suppress only the indicator overlay while retaining the tray icon and layer
socket, set this in the application's `config.toml` and restart it:

```toml
headless = true
```

Then build the extension:

```sh
cd extensions/vscode
npm install
npm run compile
```

Use VS Code's **Run Extension** launch configuration, or package the extension
with `@vscode/vsce` and install the resulting `.vsix` file.

The extension uses `$XDG_RUNTIME_DIR/qmk-layer-indicator.sock` when available,
otherwise the system temporary directory. Set
`qmkLayerIndicator.socketPath` if the daemon uses a custom path.
