# QMK Layer Indicator GNOME Shell extension

Shows the layer received from the QMK Layer Indicator Unix socket as `L0`,
`L1`, and so on in the GNOME top panel. It does not access the keyboard HID
device itself.

## Install for development

Start the QMK Layer Indicator application, then package and install the
extension. GNOME Shell may not discover symbolic links in its extensions
directory.

```sh
bundle_dir="$(mktemp -d)"
gnome-extensions pack --force --out-dir "$bundle_dir" \
  extensions/gnome-shell/qmk-layer-indicator@organic-scholar
gnome-extensions install --force \
  "$bundle_dir/qmk-layer-indicator@organic-scholar.shell-extension.zip"
```

GNOME Shell discovers newly installed extensions at session start. On Wayland,
log out and back in, then enable it:

```sh
gnome-extensions enable qmk-layer-indicator@organic-scholar
```

Repeat this install and session-restart sequence after changing extension source
files. To remove a prior
development symlink before installing the bundle:

```sh
unlink ~/.local/share/gnome-shell/extensions/qmk-layer-indicator@organic-scholar
```

The extension uses `$QMK_LAYER_INDICATOR_SOCKET` when set; otherwise it follows
the application default of `$XDG_RUNTIME_DIR/qmk-layer-indicator.sock`.
