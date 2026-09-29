# Source this file from ~/.bashrc after installing qmk-layer-indicator-shell.
if [[ $- == *i* && -t 1 ]]; then
  if [[ -z ${_QMK_LAYER_INDICATOR_PID:-} ]] || ! kill -0 "$_QMK_LAYER_INDICATOR_PID" 2>/dev/null; then
    "$HOME/.local/bin/qmk-layer-indicator-shell" "$$" </dev/null >/dev/null 2>&1 &
    _QMK_LAYER_INDICATOR_PID=$!
    disown 2>/dev/null || true
  fi
fi
