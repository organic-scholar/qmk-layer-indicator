import * as net from "node:net";
import * as os from "node:os";
import * as path from "node:path";
import * as vscode from "vscode";

const RECONNECT_DELAY_MS = 3_000;

let connection: net.Socket | undefined;
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
  statusBarItem.text = "$(keyboard) Layer —";
  statusBarItem.show();
  context.subscriptions.push(statusBarItem);

  connect();
  context.subscriptions.push({ dispose });
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (event.affectsConfiguration("qmkLayerIndicator.socketPath")) {
        connection?.destroy();
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

function handleData(data: string): void {
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

function socketPath(): string {
  const configuredPath = vscode.workspace
    .getConfiguration("qmkLayerIndicator")
    .get<string>("socketPath", "");
  if (configuredPath) {
    return configuredPath;
  }

  return path.join(process.env.XDG_RUNTIME_DIR || os.tmpdir(), "qmk-layer-indicator.sock");
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
  connection?.destroy();
  connection = undefined;
}
