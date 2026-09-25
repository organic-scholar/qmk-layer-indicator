import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Soup from 'gi://Soup?version=3.0';
import St from 'gi://St';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';

const RECONNECT_DELAY_MS = 3_000;
const DEFAULT_WEBSOCKET_URL = 'ws://127.0.0.1:51837';

Gio._promisify(Soup.Session.prototype, 'websocket_connect_async',
    'websocket_connect_finish');

export default class QmkLayerIndicatorExtension extends Extension {
    enable() {
        this._cancellable = new Gio.Cancellable();
        this._websocket = null;
        this._session = new Soup.Session();
        this._retrySource = 0;

        this._indicator = new PanelMenu.Button(0.0, 'QMK Layer Indicator');
        this._box = new St.BoxLayout({style_class: 'panel-status-menu-box'});
        this._icon = new St.Icon({
            gicon: new Gio.FileIcon({file: this.dir.get_child('icon.svg')}),
            style_class: 'system-status-icon',
            style: 'color: white;',
        });
        this._label = new St.Label({
            text: '?',
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
        this._websocket?.close(Soup.WebsocketCloseCode.NORMAL, null);
        this._websocket = null;
        this._session = null;
        this._indicator.destroy();
        this._indicator = null;
        this._box = null;
        this._icon = null;
        this._label = null;
        this._cancellable = null;
    }

    async _connect() {
        this._setConnecting();
        try {
            const request = Soup.Message.new('GET', this._webSocketUrl());
            this._websocket = await this._session.websocket_connect_async(
                request, null, null, GLib.PRIORITY_DEFAULT, this._cancellable);
            this._websocket.connect('message', (_connection, _type, message) =>
                this._handleMessage(message));
            this._websocket.connect('closed', () => {
                this._websocket = null;
                if (!this._cancellable.is_cancelled())
                    this._scheduleReconnect();
            });
        } catch (error) {
            if (!this._cancellable.is_cancelled()) {
                console.debug(`QMK Layer Indicator WebSocket unavailable: ${error.message}`);
                this._scheduleReconnect();
            }
        }
    }

    _handleMessage(message) {
        const text = new TextDecoder().decode(message.get_data());
        const match = /^LAYER:(\d+)$/.exec(text.trim());
        if (!match)
            return;

        const layer = match[1];
        this._label.text = layer;
        this._indicator.set_accessible_name(`QMK keyboard layer ${layer}`);
    }

    _setConnecting() {
        this._label.text = '?';
        this._indicator.set_accessible_name('Waiting for QMK Layer Indicator');
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

    _webSocketUrl() {
        return GLib.getenv('QMK_LAYER_INDICATOR_WEBSOCKET_URL')
            ?? DEFAULT_WEBSOCKET_URL;
    }
}
