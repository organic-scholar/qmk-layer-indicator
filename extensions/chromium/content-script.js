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
  const shape = CARET_SHAPES[layer] || "bar";
  let el = document.getElementById(STYLE_ID);
  if (!el) {
    el = document.createElement("style");
    el.id = STYLE_ID;
    el.dataset["shape"] = shape;
    (document.head || document.documentElement).append(el);
  }
  if ("shape" in el.dataset && el.dataset.shape === shape) {
    return
  }
  el.dataset.shape = shape;
  el.textContent = `
    input:not([type="button"]):not([type="checkbox"]):not([type="radio"]),
    textarea,
    [contenteditable="true"] {
      caret-shape: ${shape} !important;
    }
  `;
}
