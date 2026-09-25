import * as vscode from "vscode";
import WebSocket from "ws";

const RECONNECT_DELAY_MS = 3_000;

let connection: WebSocket | undefined;
let reconnectTimer: NodeJS.Timeout | undefined;
let received = "";
let disposed = false;
let currentLayer: string | undefined;
let currentCursorStyle: vscode.TextEditorCursorStyle | undefined;
let statusBarItem: vscode.StatusBarItem | undefined;

const cursorStyles: Record<string, vscode.TextEditorCursorStyle> = {
  line: vscode.TextEditorCursorStyle.Line,
  block: vscode.TextEditorCursorStyle.Block,
  underline: vscode.TextEditorCursorStyle.Underline,
  "line-thin": vscode.TextEditorCursorStyle.LineThin,
  "block-outline": vscode.TextEditorCursorStyle.BlockOutline,
  "underline-thin": vscode.TextEditorCursorStyle.UnderlineThin,
};

export function activate(context: vscode.ExtensionContext): void {
  statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left);
  statusBarItem.name = "QMK Layer";
  statusBarItem.tooltip = "QMK keyboard layer";
  setConnectingStatus();
  statusBarItem.show();
  context.subscriptions.push(statusBarItem);

  connect();
  context.subscriptions.push({ dispose });
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (event.affectsConfiguration("qmkLayerIndicator.webSocketUrl")) {
        connection?.terminate();
      }
      if (event.affectsConfiguration("qmkLayerIndicator.layerCursorStyles") && currentLayer) {
        updateCursorStyle(currentLayer);
      }
    }),
  );
  context.subscriptions.push(
    vscode.window.onDidChangeActiveTextEditor(() => applyCursorStyle()),
  );
}

function connect(): void {
  if (disposed || connection) {
    return;
  }

  setConnectingStatus();
  received = "";
  connection = new WebSocket(webSocketUrl());
  connection.on("message", handleData);
  connection.on("error", () => connection?.close());
  connection.on("close", () => {
    connection = undefined;
    setConnectingStatus();
    scheduleReconnect();
  });
}

function handleData(data: WebSocket.RawData): void {
  received += data.toString();
  const lines = received.split("\n");
  received = lines.pop() ?? "";

  for (const line of lines) {
    const match = /^LAYER:(\d+)$/.exec(line.trim());
    if (match) {
      updateCursorStyle(match[1]);
    }
  }
}

function updateCursorStyle(layer: string): void {
  currentLayer = layer;
  updateStatusBar(layer);
  const styles = vscode.workspace
    .getConfiguration("qmkLayerIndicator")
    .get<Record<string, string>>("layerCursorStyles", {});
  const style = cursorStyles[styles[layer]];
  if (!style) {
    return;
  }

  currentCursorStyle = style;
  applyCursorStyle();
}

function updateStatusBar(layer: string): void {
  if (!statusBarItem) {
    return;
  }

  statusBarItem.text = `$(keyboard) Layer ${layer}`;
  statusBarItem.tooltip = `QMK keyboard layer ${layer}`;
}

function setConnectingStatus(): void {
  if (!statusBarItem) {
    return;
  }

  statusBarItem.text = "$(keyboard) Layer ?";
  statusBarItem.tooltip = "Waiting for QMK Layer Indicator";
}

function applyCursorStyle(): void {
  if (!currentCursorStyle) {
    return;
  }

  for (const editor of vscode.window.visibleTextEditors) {
    editor.options.cursorStyle = currentCursorStyle;
  }
}

function scheduleReconnect(): void {
  if (disposed || reconnectTimer) {
    return;
  }
  reconnectTimer = setTimeout(() => {
    reconnectTimer = undefined;
    connect();
  }, RECONNECT_DELAY_MS);
}

function webSocketUrl(): string {
  return vscode.workspace
    .getConfiguration("qmkLayerIndicator")
    .get<string>("webSocketUrl", "ws://127.0.0.1:51837");
}

export function deactivate(): void {
  dispose();
}

function dispose(): void {
  disposed = true;
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
    reconnectTimer = undefined;
  }
  connection?.terminate();
  connection = undefined;
}
