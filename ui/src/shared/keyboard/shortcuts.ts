// The one list of shortcut keys (docs/07 UI Design/Keyboard Shortcuts.md).
// Screens add their keys here as they are built, so no two actions share a key.
// `label` is what the screen shows; `keys` is the same key for react-hotkeys-hook.
export const shortcuts = {
  help: "F1",
  // Locks the till; the cart is kept (USR-03).
  lock: { label: "F12", keys: "f12" },
  // Saves the form on screen (back office).
  save: { label: "Ctrl+S", keys: "ctrl+s" },
  // Leaves a back-office form without saving.
  back: { label: "Esc", keys: "escape" },
  // Checkout (Keyboard Shortcuts, "Checkout"). Function keys never clash with scanners.
  checkoutSearch: { label: "F2", keys: "f2" },
  checkoutQty: { label: "F4", keys: "f4" },
  checkoutHold: { label: "F5", keys: "f5" },
  checkoutHeld: { label: "F6", keys: "f6" },
  checkoutCancel: { label: "F9", keys: "f9" },
  checkoutPay: { label: "F10", keys: "f10" },
  checkoutShift: { label: "F11", keys: "f11" },
  checkoutRemove: { label: "Del", keys: "delete" },
  // Payment dialog: quick cash amounts.
  quickCash: ["F1", "F2", "F3", "F4"],
  // Confirms a dialog (Complete, Change, Add).
  confirm: { label: "Enter", keys: "enter" },
  // Dev builds only (Dev Environment Setup, "Working without hardware").
  scanSimulator: { label: "Ctrl+Shift+S", keys: "ctrl+shift+s" },
};
