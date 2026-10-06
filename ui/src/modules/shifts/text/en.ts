// Every piece of text on the shift dialogs (ADR-022). Plain words (Writing Style).
export const en = {
  count: {
    title: "Count by note and coin",
    each: {
      label: "{{note}}",
      tip: "How many {{note}} notes or coins. Example: 12",
    },
    total: "Counted total",
  },
  open: {
    title: "Open a shift",
    intro: "Count the money in the drawer before you start. Counting by note is optional.",
    float: {
      label: "Opening float",
      tip: "The cash in the drawer now. Example: 2000.00",
    },
    countInstead: {
      label: "Count by note",
      tip: "Count each note and coin instead of typing the total.",
    },
    back: { label: "Back", tip: "Close without opening a shift." },
    save: { label: "Open shift", tip: "Start your shift with this float." },
  },
  menu: {
    title: "Shift",
    cashIn: {
      label: "Cash in",
      tip: "Put cash into the drawer, for example extra change. Needs an admin.",
    },
    cashOut: {
      label: "Paid out",
      tip: "Pay cash out of the drawer, for example to a delivery person. Needs an admin.",
    },
    cashDrop: { label: "Cash drop", tip: "Take cash from the drawer to the safe. Needs an admin." },
    noSale: { label: "Open drawer", tip: "Open the drawer without a sale. Needs an admin." },
    xReport: {
      label: "X report",
      tip: "Print the totals of your shift so far. The shift stays open.",
    },
    close: { label: "Close shift", tip: "Count the drawer and close your shift." },
    back: { label: "Back", tip: "Go back to the till." },
    done: "Saved.",
    xPrinted: "X report printed.",
  },
  move: {
    titleCashIn: "Cash in",
    titleCashOut: "Paid out",
    titleCashDrop: "Cash drop",
    titleNoSale: "Open the drawer",
    amount: { label: "Amount", tip: "How much cash. Example: 500.00" },
    reason: { label: "Reason", tip: "Why the cash moves. Example: delivery man for rice" },
    back: { label: "Back", tip: "Close without saving." },
    save: { label: "Save", tip: "Save this and write it to the audit log." },
  },
  close: {
    title: "Close shift",
    forceTitle: "Force close the shift",
    intro: "Count every note and coin in the drawer.",
    forceIntro:
      "This closes the shift left open on this till. The count is optional. An open sale is held.",
    expected: "Expected cash: {{amount}}",
    note: {
      label: "Note (optional)",
      tip: "Anything to explain. Example: 50 short, a note was torn",
    },
    back: { label: "Back", tip: "Close without closing the shift." },
    save: { label: "Close shift", tip: "Close the shift with this count." },
    forceSave: { label: "Force close", tip: "Close the shift. Needs an admin." },
  },
  closed: {
    title: "Shift closed",
    expected: "Expected",
    counted: "Counted",
    difference: "Difference",
    short: "Short",
    over: "Over",
    exact: "Exact",
    notCounted: "Not counted",
    zPrinted: "Z report printed.",
    ok: { label: "Done", tip: "Close this summary." },
  },
};
