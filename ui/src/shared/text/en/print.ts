// Screen text for the banner shown when a receipt, report or drawer pulse fails
// (PAY-09, HW-07, HW-08), in plain English (ADR-022).
export const print = {
  receiptFailed: "Receipt not printed. The sale is saved.",
  reportFailed: "Report not printed. The shift is saved.",
  drawerFailed: "The cash drawer did not open. Open it with its key.",
  retry: { label: "Retry", tip: "Send the printout to the receipt printer again." },
  system: {
    label: "Print to system printer",
    tip: "Print it on a normal printer through the computer's print dialog.",
  },
  dismiss: { label: "Close", tip: "Hide this message. You can reprint the receipt later." },
  printed: "Printed.",
  pictureAlt: "The printout",
};

export type PrintText = typeof print;
