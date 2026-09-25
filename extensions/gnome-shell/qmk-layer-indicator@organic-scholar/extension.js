import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import St from 'gi://St';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';

const RECONNECT_DELAY_MS = 3_000;
const SOCKET_NAME = 'qmk-layer-indicator.sock';

Gio._promisify(Gio.DataInputStream.prototype, 'read_line_async',
    'read_line_finish_utf8');
Gio._promisify(Gio.SocketClient.prototype, 'connect_async');

export default class QmkLayerIndicatorExtension extends Extension {
    enable() {
        this._cancellable = new Gio.Cancellable();
        this._connection = null;
        this._retrySource = 0;

        this._indicator = new PanelMenu.Button(0.0, 'QMK Layer Indicator');
        this._box = new St.BoxLayout({style_class: 'panel-status-menu-box'});
        this._icon = new St.Icon({
            gicon: new Gio.FileIcon({file: this.dir.get_child('icon.svg')}),
            style_class: 'system-status-icon',
            style: 'color: white;',
        });
        this._label = new St.Label({
            text: '—',
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._box.add_child(this._icon);
        this._box.add_child(this._label);
        this._indicator.add_child(this._box);
        this._indicator.set_accessible_name('Waiting for QMK Layer Indicator');
        Main.panel.addToStatusArea(this.uuid, this._indicator, 0, 'right');

        void this._connect();
    }

    disable() {
        this._cancellable.cancel();
        if (this._retrySource) {
            GLib.Source.remove(this._retrySource);
            this._retrySource = 0;
        }
        this._connection?.close(null);
        this._connection = null;
        this._indicator.destroy();
        this._indicator = null;
        this._box = null;
        this._icon = null;
        this._label = null;
        this._cancellable = null;
    }

    async _connect() {
        try {
            const client = new Gio.SocketClient();
            const address = Gio.UnixSocketAddress.new(this._socketPath());
            this._connection = await client.connect_async(address, this._cancellable);
            const input = new Gio.DataInputStream({
                base_stream: this._connection.get_input_stream(),
            });

            while (!this._cancellable.is_cancelled()) {
                const [line] = await input.read_line_async(
                    GLib.PRIORITY_DEFAULT, this._cancellable);
                if (line === null)
                    break;
                this._handleMessage(line);
            }
        } catch (error) {
            console.log(error)
            if (!this._cancellable.is_cancelled())
                console.debug(`QMK Layer Indicator socket unavailable: ${error.message}`);
        } finally {
            this._connection?.close(null);
            this._connection = null;
            if (!this._cancellable.is_cancelled())
                this._scheduleReconnect();
        }
    }

    _handleMessage(message) {
        const text = typeof message === 'string'
            ? message
            : new TextDecoder().decode(message);
        const match = /^LAYER:(\d+)$/.exec(text.trim());
        if (!match)
            return;

        const layer = match[1];
        this._label.text = layer;
        this._indicator.set_accessible_name(`QMK keyboard layer ${layer}`);
    }

    _scheduleReconnect() {
        if (this._retrySource)
            return;

        this._retrySource = GLib.timeout_add(GLib.PRIORITY_DEFAULT,
            RECONNECT_DELAY_MS, () => {
                this._retrySource = 0;
                void this._connect();
                return GLib.SOURCE_REMOVE;
            });
    }

    _socketPath() {
        const configuredPath = GLib.getenv('QMK_LAYER_INDICATOR_SOCKET');
        if (configuredPath)
            return configuredPath;

        const runtimeDirectory = GLib.getenv('XDG_RUNTIME_DIR') ?? GLib.get_tmp_dir();
        return GLib.build_filenamev([runtimeDirectory, SOCKET_NAME]);
    }
}
