// The one list of shortcut keys (docs/07 UI Design/Keyboard Shortcuts.md).
// Screens add their keys here as they are built, so no two actions share a key.
// `label` is what the screen shows; `keys` is the same key for react-hotkeys-hook.
export const shortcuts = {
  help: "F1",
  // Locks the till; the cart is kept (USR-03).
  lock: { label: "F12", keys: "f12" },
  // Dev builds only (Dev Environment Setup, "Working without hardware").
  scanSimulator: { label: "Ctrl+Shift+S", keys: "ctrl+shift+s" },
};
