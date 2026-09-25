# QMK Layer Indicator for Chrome and Brave

This Manifest V3 extension changes the insertion-caret shape in ordinary web
page text inputs, textareas, and editable elements in response to QMK layer
events:

| Layer | Caret |
| --- | --- |
| 0 | bar |
| 1 | block |
| 2 | underscore |
| other | bar |

It cannot alter the browser's address bar, DevTools, PDF viewer, or other
browser-owned UI. The CSS `caret-shape` property requires a recent Chromium
browser.

## Install for development

1. Visit `chrome://extensions` in Chrome or `brave://extensions` in Brave.
2. Enable **Developer mode**.
3. Choose **Load unpacked** and select this `extensions/chromium` directory.
4. Copy the displayed extension ID.

Start `qmk-layer-indicator` normally or with `headless = true`; either mode
starts a WebSocket server at `ws://127.0.0.1:51837`. No Native Messaging host
or browser-specific installation is required.
