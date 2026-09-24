"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
exports.deactivate = deactivate;
const net = __importStar(require("node:net"));
const os = __importStar(require("node:os"));
const path = __importStar(require("node:path"));
const vscode = __importStar(require("vscode"));
const RECONNECT_DELAY_MS = 3_000;
let connection;
let reconnectTimer;
let received = "";
let disposed = false;
let currentLayer;
let currentCursorStyle;
let statusBarItem;
const cursorStyles = {
    line: vscode.TextEditorCursorStyle.Line,
    block: vscode.TextEditorCursorStyle.Block,
    underline: vscode.TextEditorCursorStyle.Underline,
    "line-thin": vscode.TextEditorCursorStyle.LineThin,
    "block-outline": vscode.TextEditorCursorStyle.BlockOutline,
    "underline-thin": vscode.TextEditorCursorStyle.UnderlineThin,
};
function activate(context) {
    statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left);
    statusBarItem.name = "QMK Layer";
    statusBarItem.tooltip = "QMK keyboard layer";
    statusBarItem.text = "$(keyboard) Layer —";
    statusBarItem.show();
    context.subscriptions.push(statusBarItem);
    connect();
    context.subscriptions.push({ dispose });
    context.subscriptions.push(vscode.workspace.onDidChangeConfiguration((event) => {
        if (event.affectsConfiguration("qmkLayerIndicator.socketPath")) {
            connection?.destroy();
        }
        if (event.affectsConfiguration("qmkLayerIndicator.layerCursorStyles") && currentLayer) {
            updateCursorStyle(currentLayer);
        }
    }));
    context.subscriptions.push(vscode.window.onDidChangeActiveTextEditor(() => applyCursorStyle()));
}
function connect() {
    if (disposed || connection) {
        return;
    }
    received = "";
    connection = net.createConnection(socketPath());
    connection.setEncoding("utf8");
    connection.on("data", handleData);
    connection.on("error", () => connection?.destroy());
    connection.on("close", () => {
        connection = undefined;
        scheduleReconnect();
    });
}
function handleData(data) {
    received += data;
    const lines = received.split("\n");
    received = lines.pop() ?? "";
    for (const line of lines) {
        const match = /^LAYER:(\d+)$/.exec(line.trim());
        if (match) {
            updateCursorStyle(match[1]);
        }
    }
}
function updateCursorStyle(layer) {
    currentLayer = layer;
    updateStatusBar(layer);
    const styles = vscode.workspace
        .getConfiguration("qmkLayerIndicator")
        .get("layerCursorStyles", {});
    const style = cursorStyles[styles[layer]];
    if (!style) {
        return;
    }
    currentCursorStyle = style;
    applyCursorStyle();
}
function updateStatusBar(layer) {
    if (!statusBarItem) {
        return;
    }
    statusBarItem.text = `$(keyboard) Layer ${layer}`;
    statusBarItem.tooltip = `QMK keyboard layer ${layer}`;
}
function applyCursorStyle() {
    if (!currentCursorStyle) {
        return;
    }
    for (const editor of vscode.window.visibleTextEditors) {
        editor.options.cursorStyle = currentCursorStyle;
    }
}
function scheduleReconnect() {
    if (disposed || reconnectTimer) {
        return;
    }
    reconnectTimer = setTimeout(() => {
        reconnectTimer = undefined;
        connect();
    }, RECONNECT_DELAY_MS);
}
function socketPath() {
    const configuredPath = vscode.workspace
        .getConfiguration("qmkLayerIndicator")
        .get("socketPath", "");
    if (configuredPath) {
        return configuredPath;
    }
    return path.join(process.env.XDG_RUNTIME_DIR || os.tmpdir(), "qmk-layer-indicator.sock");
}
function deactivate() {
    dispose();
}
function dispose() {
    disposed = true;
    if (reconnectTimer) {
        clearTimeout(reconnectTimer);
        reconnectTimer = undefined;
    }
    connection?.destroy();
    connection = undefined;
}
//# sourceMappingURL=extension.js.map