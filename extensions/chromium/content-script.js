const STYLE_ID = "qmk-layer-indicator-caret-shape";
const CARET_SHAPES = {
  0: "bar",
  1: "block",
  2: "underscore",
};

chrome.runtime.onMessage.addListener(message => {
  if (message.type === "qmk-layer-indicator:layer") {
    setCaretShape(message.layer);
  }
});

chrome.runtime.sendMessage({ type: "qmk-layer-indicator:get-state" })
  .then(({ layer }) => {
    if (Number.isInteger(layer)) {
      setCaretShape(layer);
    }
  })
  .catch(() => {});

function setCaretShape(layer) {
  const shape = CARET_SHAPES[layer] ?? "bar";
  let style = document.getElementById(STYLE_ID);
  if (!style) {
    style = document.createElement("style");
    style.id = STYLE_ID;
    (document.head || document.documentElement).append(style);
  }

  style.textContent = `
    input:not([type="button"]):not([type="checkbox"]):not([type="radio"]),
    textarea,
    [contenteditable="true"] {
      caret-shape: ${shape} !important;
    }
  `;
}
