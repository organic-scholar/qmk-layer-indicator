# Shell cursor integration

This extension keeps a background listener for each interactive bash or zsh
terminal. It updates the cursor as soon as the QMK layer changes:

| Layer | Cursor |
| --- | --- |
| 0 and other layers | bar |
| 1 | block |
| 2 | underline |

It uses the main application's WebSocket at `ws://127.0.0.1:51837`. Set
`QMK_LAYER_INDICATOR_URL` to use a different URL. The terminal must support the
DECSCUSR cursor-shape escape sequence. The listener reconnects automatically
when the application restarts or is temporarily unavailable, and exits when
its shell exits.

Install for your login shell from the repository root:

```sh
just install-shell
```

Pass `bash` or `zsh` to install for a different shell, for example
`just install-shell bash`. The recipe installs the helper in `~/.local/bin`,
copies the matching hook to `~/.local/share/qmk-layer-indicator/shell`, and
prints the source line to add to `~/.bashrc` or `~/.zshrc`. Add that line
yourself, then open a new shell. The recipe does not edit either rc file.

For a manual setup, build the helper and source one of the hooks:

```sh
cargo build --locked --release --bin qmk-layer-indicator-shell
mkdir -p "$HOME/.local/bin"
install -m 755 target/release/qmk-layer-indicator-shell "$HOME/.local/bin/qmk-layer-indicator-shell"

# ~/.zshrc
source /path/to/qmk-layer-indicator/extensions/shell/qmk-layer-indicator.zsh

# ~/.bashrc
source /path/to/qmk-layer-indicator/extensions/shell/qmk-layer-indicator.bash
```

The QMK Layer Indicator service must be running to receive layer changes. If
it is unavailable, the listener keeps retrying and leaves the current cursor
shape unchanged until it reconnects.
