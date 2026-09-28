# QMK Layer Indicator for VS Code

This extension connects to the QMK Layer Indicator application and updates the
in-memory cursor style of visible VS Code editors when it receives a layer event.
It does not modify `settings.json` or the user's `editor.cursorStyle` preference.
It also shows the current QMK layer in VS Code's status bar.
If the app config defines an alias, the status bar shows that alias and the
tooltip includes the numeric layer.

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

Start the background service. Its layer WebSocket server is then available:

```sh
qmk-layer-indicator
```

Then build the extension:

```sh
cd extensions/vscode
npm install
npm run compile
```

Use VS Code's **Run Extension** launch configuration, or package the extension
with `@vscode/vsce` and install the resulting `.vsix` file.

The extension uses `ws://127.0.0.1:51837` by default. Set
`qmkLayerIndicator.webSocketUrl` to use a different local server URL.
Each WebSocket event is JSON, for example `{"layer":1,"alias":"S"}`. An
unconfigured alias is an empty string.
