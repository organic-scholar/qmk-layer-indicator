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
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
exports.deactivate = deactivate;
const vscode = __importStar(require("vscode"));
const ws_1 = __importDefault(require("ws"));
const RECONNECT_DELAY_MS = 3_000;
let connection;
let reconnectTimer;
let disposed = false;
let currentLayer;
let currentAlias = "";
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
    setConnectingStatus();
    statusBarItem.show();
    context.subscriptions.push(statusBarItem);
    connect();
    context.subscriptions.push({ dispose });
    context.subscriptions.push(vscode.workspace.onDidChangeConfiguration((event) => {
        if (event.affectsConfiguration("qmkLayerIndicator.webSocketUrl")) {
            connection?.terminate();
        }
        if (event.affectsConfiguration("qmkLayerIndicator.layerCursorStyles") && currentLayer) {
            updateCursorStyle(currentLayer, currentAlias);
        }
    }));
    context.subscriptions.push(vscode.window.onDidChangeActiveTextEditor(() => applyCursorStyle()));
}
function connect() {
    if (disposed || connection) {
        return;
    }
    setConnectingStatus();
    connection = new ws_1.default(webSocketUrl());
    connection.on("message", handleData);
    connection.on("error", () => connection?.close());
    connection.on("close", () => {
        connection = undefined;
        setConnectingStatus();
        scheduleReconnect();
    });
}
function handleData(data) {
    try {
        const message = JSON.parse(data.toString());
        if (typeof message !== "object" || message === null) {
            return;
        }
        const { layer, alias } = message;
        if (Number.isInteger(layer) && typeof layer === "number" && layer >= 0 && typeof alias === "string") {
            updateCursorStyle(String(layer), alias);
        }
    }
    catch {
        // Ignore malformed messages.
    }
}
function updateCursorStyle(layer, alias) {
    currentLayer = layer;
    currentAlias = alias;
    updateStatusBar(layer, alias);
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
function updateStatusBar(layer, alias) {
    if (!statusBarItem) {
        return;
    }
    statusBarItem.text = `$(keyboard) Layer ${alias || layer}`;
    statusBarItem.tooltip = alias
        ? `QMK keyboard layer ${layer}: ${alias}`
        : `QMK keyboard layer ${layer}`;
}
function setConnectingStatus() {
    if (!statusBarItem) {
        return;
    }
    statusBarItem.text = "$(keyboard) Layer ?";
    statusBarItem.tooltip = "Waiting for QMK Layer Indicator";
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
function webSocketUrl() {
    return vscode.workspace
        .getConfiguration("qmkLayerIndicator")
        .get("webSocketUrl", "ws://127.0.0.1:51837");
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
    connection?.terminate();
    connection = undefined;
}
//# sourceMappingURL=extension.js.map