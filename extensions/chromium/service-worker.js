const RECONNECT_DELAY_MS = 3_000;
const WEBSOCKET_URL = "ws://127.0.0.1:51837";

let socket;
let reconnectTimer;
let currentLayer;

connectWebSocket();

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message.type === "qmk-layer-indicator:get-state") {
    sendResponse({ layer: currentLayer });
  }
});

function connectWebSocket() {
  if (socket) {
    return;
  }

  socket = new WebSocket(WEBSOCKET_URL);
  socket.addEventListener("message", event => handleLayerEvent(event.data));
  socket.addEventListener("close", () => {
    socket = undefined;
    scheduleReconnect();
  });
  socket.addEventListener("error", () => socket?.close());
}

function handleLayerEvent(event) {
  const match = /^LAYER:(\d+)$/.exec(event);
  if (!match) {
    return;
  }

  currentLayer = Number(match[1]);
  chrome.tabs.query({}, tabs => {
    for (const tab of tabs) {
      if (tab.id !== undefined) {
        chrome.tabs.sendMessage(tab.id, {
          type: "qmk-layer-indicator:layer",
          layer: currentLayer,
        }).catch(() => {});
      }
    }
  });
}

function scheduleReconnect() {
  if (reconnectTimer) {
    return;
  }

  reconnectTimer = setTimeout(() => {
    reconnectTimer = undefined;
    connectWebSocket();
  }, RECONNECT_DELAY_MS);
}
