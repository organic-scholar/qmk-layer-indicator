set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

app_name := "QMK Layer Indicator"
bundle_id := "com.organic-scholar.qmk-layer-indicator"
version := `sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n1`

default:
    @just --list

# Create the committed release source archive consumed by the PKGBUILD.
arch-source:
    mkdir -p packaging/arch/dist
    git archive --format=tar.gz --prefix="qmk-layer-indicator-{{version}}/" HEAD > "packaging/arch/dist/qmk-layer-indicator-{{version}}.tar.gz"

# Build an Arch package. Pass makepkg options after the recipe name, e.g.
# `just arch-package --nodeps` when using a Rustup toolchain locally.
arch-package *makepkg_args: arch-source
    grep -qx 'pkgver={{version}}' packaging/arch/PKGBUILD || { echo 'Update pkgver in packaging/arch/PKGBUILD first.' >&2; exit 1; }
    cd packaging/arch && SRCDEST="$PWD/dist" PKGDEST="$PWD/dist" makepkg --cleanbuild {{makepkg_args}}

# Build a DMG for the Mac architecture on which this recipe is run.
macos-package:
    #!/usr/bin/env bash
    [[ "$(uname)" == "Darwin" ]] || { echo 'This recipe must run on macOS.' >&2; exit 1; }
    app="packaging/macos/dist/{{app_name}}.app"
    dmg="packaging/macos/dist/qmk-layer-indicator-{{version}}-$(uname -m).dmg"
    iconset="$(mktemp -d)"
    trap 'rm -rf "$iconset"' EXIT
    cargo build --locked --release
    mkdir -p packaging/macos/dist
    rm -rf "$app"
    mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
    cp target/release/qmk-layer-indicator "$app/Contents/MacOS/qmk-layer-indicator"
    for size in 16 32 128 256 512; do
        sips -z "$size" "$size" assets/icon.png --out "$iconset/icon_${size}x${size}.png"
        double=$((size * 2))
        sips -z "$double" "$double" assets/icon.png --out "$iconset/icon_${size}x${size}@2x.png"
    done
    iconutil -c icns "$iconset" -o "$app/Contents/Resources/qmk-layer-indicator.icns"
    sed 's/@VERSION@/{{version}}/g' packaging/macos/Info.plist.in > "$app/Contents/Info.plist"
    hdiutil create -volname "{{app_name}}" -srcfolder "$app" -ov -format UDZO "$dmg"

# Install a per-user LaunchAgent for an app bundle installed in /Applications.
macos-install-login-agent app="/Applications/QMK Layer Indicator.app":
    #!/usr/bin/env bash
    [[ "$(uname)" == "Darwin" ]] || { echo 'This recipe must run on macOS.' >&2; exit 1; }
    executable="{{app}}/Contents/MacOS/qmk-layer-indicator"
    [[ -x "$executable" ]] || { echo "App executable not found: $executable" >&2; exit 1; }
    plist="$HOME/Library/LaunchAgents/{{bundle_id}}.plist"
    uid="$(id -u)"
    mkdir -p "$(dirname "$plist")"
    launchctl bootout "gui/$uid" "$plist" 2>/dev/null || true
    sed "s|@EXECUTABLE@|$executable|g" packaging/macos/launch-agent.plist.in > "$plist"
    plutil -lint "$plist"
    launchctl bootstrap "gui/$uid" "$plist"

macos-remove-login-agent:
    #!/usr/bin/env bash
    [[ "$(uname)" == "Darwin" ]] || { echo 'This recipe must run on macOS.' >&2; exit 1; }
    plist="$HOME/Library/LaunchAgents/{{bundle_id}}.plist"
    launchctl bootout "gui/$(id -u)" "$plist" 2>/dev/null || true
    rm -f "$plist"
