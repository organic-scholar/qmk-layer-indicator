const RECONNECT_DELAY_MS = 3_000;
const RECONNECT_ALARM = "qmk-layer-indicator-reconnect";
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

chrome.runtime.onStartup.addListener(connectWebSocket);
chrome.alarms.onAlarm.addListener(alarm => {
  if (alarm.name === RECONNECT_ALARM) {
    connectWebSocket();
  }
});

function connectWebSocket() {
  if (socket) {
    return;
  }

  socket = new WebSocket(WEBSOCKET_URL);
  socket.addEventListener("open", () => {
    void chrome.alarms.clear(RECONNECT_ALARM);
  });
  socket.addEventListener("message", event => handleLayerEvent(event.data));
  socket.addEventListener("close", () => {
    socket = undefined;
    scheduleReconnect();
  });
  socket.addEventListener("error", () => socket?.close());
}

function handleLayerEvent(event) {
  console.log(event)
  let message;
  try {
    message = JSON.parse(event);
  } catch {
    return;
  }
  if (!Number.isInteger(message?.layer) || message.layer < 0 || typeof message.alias !== "string") {
    return;
  }

  currentLayer = message.layer;
  chrome.tabs.query({}, tabs => {
    for (const tab of tabs) {
      if (tab.id !== undefined) {
        chrome.tabs.sendMessage(tab.id, {
          type: "qmk-layer-indicator:layer",
          layer: currentLayer,
          alias: message.alias,
        }).catch(() => {});
      }
    }
  });
}

function scheduleReconnect() {
  void chrome.alarms.create(RECONNECT_ALARM, {
    delayInMinutes: 0.5,
    periodInMinutes: 0.5,
  });
  if (reconnectTimer) {
    return;
  }

  reconnectTimer = setTimeout(() => {
    reconnectTimer = undefined;
    connectWebSocket();
  }, RECONNECT_DELAY_MS);
}
