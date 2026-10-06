// All screen text for the printer and device screens, in plain English (ADR-022).
export const en = {
  page: {
    title: "Printer and devices",
    intro:
      "Test print goes to the receipt printer chosen in Settings. With the Preview printer, printouts show here exactly as they would print.",
  },
  testPrint: {
    heading: "Test print",
    paperLabel: "Paper width",
    mm58: {
      label: "58 mm",
      tip: "Print on narrow paper, 58 mm wide (about 32 letters a line).",
    },
    mm80: {
      label: "80 mm",
      tip: "Print on wide paper, 80 mm wide (about 48 letters a line).",
    },
    button: {
      label: "Test print",
      tip: "Print a sample receipt with text, the currency sign, a logo, a barcode and a QR code.",
    },
    busy: "Printing…",
    sent: "The test receipt was sent to the printer.",
  },
  tillPreview: {
    heading: "Preview printer",
    pictureAlt: "The newest printout",
    close: { label: "Close", tip: "Hide the printout. The next one shows here again." },
  },
  preview: {
    heading: "Preview printer",
    intro: "No printer is connected, so each printout shows here as the exact picture.",
    empty: "Nothing printed yet. Press Test print.",
    pictureAlt: "Printout {{number}}",
    drawerOpened: "Drawer opened",
    drawerTip: "A real printer would have opened the cash drawer now.",
    savedTo: "Saved as a picture: {{file}}",
    printButton: {
      label: "Print on another printer",
      tip: "Open the computer's print dialog to print this on a normal printer or save it as a PDF.",
    },
    clearButton: {
      label: "Clear",
      tip: "Remove these printouts from the screen. The saved pictures stay.",
    },
  },
  scale: {
    heading: "Fake scale",
    intro: "For trying weighed items without a scale. Only in test builds.",
    weight: {
      label: "Weight",
      tip: "The weight on the scale, with up to 3 decimals. Example: 1.250",
    },
    unit: {
      label: "Unit",
      kg: "kg",
      g: "g",
      lb: "lb",
      tip: "The unit the scale reports in.",
    },
    stable: {
      label: "Weight is still",
      tip: "Untick to make the weight move, like a customer still putting things down.",
    },
    connected: {
      label: "Scale plugged in",
      tip: "Untick to make the scale act unplugged. Reading it then fails after 2 seconds.",
    },
    saveButton: {
      label: "Set weight",
      tip: "Put this weight on the fake scale.",
    },
    readButton: {
      label: "Read scale",
      tip: "Read the scale the way checkout will. Shows the weight and what the scale sent.",
    },
    readingStable: "Still",
    readingMoving: "Moving",
    readingRaw: "The scale sent: {{raw}}",
    busy: "Reading the scale…",
  },
  scanner: {
    heading: "Scan simulator",
    intro:
      "Click a code to scan it into the box that last had the cursor, as fast as a real scanner. Only in test builds.",
    openButton: {
      label: "Scan simulator",
      tip: "Show or hide the scan simulator.",
    },
    codeButtonTip: "Scan {{code}} into the box that last had the cursor.",
    typed: {
      label: "Your own code",
      tip: "Type any code to scan. Example: 5901234123457",
    },
    scanButton: {
      label: "Scan",
      tip: "Scan the code you typed.",
    },
    noTarget: "Click in a box first, then pick a code.",
    samples: {
      product: "Product barcode (EAN-13)",
      small: "Small packet (EAN-8)",
      weight: "Weight sticker, 1.250 kg",
      price: "Price sticker, 99.50",
      sku: "Product code (SKU)",
      wrong: "Wrong check digit",
    },
  },
  scanBox: {
    heading: "Scan test box",
    field: {
      label: "Scan here",
      tip: "Scan or type a code and press Enter. Example: 5901234123457",
    },
    received: "Received {{code}} in {{ms}} ms",
  },
};

export type HardwareText = typeof en;
