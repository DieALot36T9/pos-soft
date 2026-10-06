// All screen text for the Settings screens, in plain English (ADR-022).
export const en = {
  page: {
    title: "Settings",
    intro: "Changes are saved per section and written to the audit log.",
    sections: "Settings sections",
  },
  sections: {
    shop: "Shop",
    money: "Money and rounding",
    tax: "Tax",
    till: "This till",
    receipt: "Receipt",
    payments: "Payment methods",
    backup: "Backups",
    logs: "Logs",
  },
  sectionTip: "Open the {{name}} settings.",
  save: { label: "Save", tip: "Save the changes in this section.", busy: "Saving…" },
  saved: "Saved.",
  payments: {
    intro:
      "The ways customers can pay at the till. Methods are never deleted: switch one off instead.",
    table: { name: "Name", kind: "Kind", reference: "Reference", active: "On" },
    yes: "Yes",
    no: "No",
    editTip: "Change {{name}}.",
    newMethod: {
      label: "New method",
      tip: "Add a payment method, such as a card machine or a mobile wallet.",
    },
    name: { label: "Name", tip: "What the cashier sees. Example: bKash" },
    kind: { label: "Kind", tip: "Decides the rules and the colour. Only cash may give change." },
    kinds: {
      cash: "Cash",
      card: "Card",
      wallet: "Mobile wallet",
      other: "Other",
      credit: "Customer credit",
      points: "Loyalty points",
      exchange_credit: "Exchange credit",
    },
    reference: {
      label: "Needs a reference",
      tip: "The cashier must type a transaction ID or slip number. Never a card number.",
    },
    active: {
      label: "Switched on",
      tip: "Shown at the till. Switch off a method the shop no longer takes.",
    },
  },
  shop: {
    name: { label: "Shop name", tip: "The name customers know. Example: Rahim Store" },
    address: { label: "Address", tip: "One line of the address. Example: 12 Lake Road, Dhaka" },
    phone: { label: "Phone", tip: "The shop's phone number. Example: 01711-000000" },
    email: { label: "Email", tip: "Optional. Example: shop@example.com" },
    taxRegLabel: { label: "Tax number name", tip: "What your tax number is called. Example: BIN" },
    taxRegNo: { label: "Tax number", tip: "Your tax registration number. Example: 000123456-0101" },
  },
  money: {
    symbol: { label: "Currency sign", tip: "The sign printed with amounts. Example: ৳" },
    symbolBefore: { label: "Sign before the amount", tip: "Tick for ৳100, untick for 100 ৳." },
    digits: {
      label: "Decimals",
      tip: "Digits after the point. Locked after the first sale.",
    },
    grouping: {
      label: "Digit grouping",
      tip: "How big numbers are split.",
      indian: "Indian (12,34,567)",
      western: "Western (1,234,567)",
    },
    preview: "1234567.5 is written as",
    roundingStep: {
      label: "Round cash to",
      tip: "Round totals to this amount. Example: 1.00. Use 0 for no rounding.",
    },
    roundingMode: {
      label: "Direction",
      tip: "Which way totals are rounded.",
      nearest: "To the nearest",
      down: "Down (customer pays less)",
      up: "Up (customer pays more)",
    },
    appliesTo: {
      label: "Applies to",
      tip: "Round only cash payments, or every payment.",
      cash_only: "Cash payments only",
      all_payments: "All payments",
    },
  },
  tax: {
    label: { label: "Tax name", tip: "What your tax is called. Example: VAT, GST" },
    mode: {
      label: "Shelf prices",
      tip: "Do the prices on the shelf include tax?",
      inclusive: "Include tax",
      exclusive: "Don't include tax",
    },
    modeWarning: "Changing this affects only future sales. Past sales keep their amounts.",
    defaultRate: { label: "Default rate", tip: "The rate new products get." },
    ratesNote: "Add or change rates under Tax rates.",
  },
  till: {
    code: { label: "Till code", tip: "1 to 6 letters or digits. Example: T1" },
    cutoff: {
      label: "New day starts at",
      tip: "Sales before this hour count for the day before. Example: 4 means 04:00.",
    },
    hour: "{{hour}}:00",
  },
  receipt: {
    paper: {
      label: "Paper width",
      tip: "The width of the receipt roll.",
      mm58: "58 mm",
      mm80: "80 mm",
    },
    copies: { label: "Copies", tip: "How many copies of each receipt to print, 1 to 3." },
    showLogo: { label: "Print the logo", tip: "Print the shop logo at the top." },
    showTaxBreakdown: { label: "Print the tax summary", tip: "Print net and tax per rate." },
    showTaxRegNo: { label: "Print the tax number", tip: "Print the shop's tax number." },
    showInvoiceBarcode: {
      label: "Print the invoice barcode",
      tip: "Scan it later to find the sale for a return.",
    },
    showCustomerBalance: {
      label: "Print customer points and balance",
      tip: "For customers with an account.",
    },
    header: {
      label: "Header line",
      tip: "A line under the shop name. Example: Open every day 8am to 11pm",
    },
    footer: {
      label: "Footer line",
      tip: "A line at the bottom. Example: Thank you for shopping with us",
    },
    previewNote: "A live picture of the receipt comes with the printer settings.",
  },
  backup: {
    folder: {
      label: "Backup folder",
      tip: "Where backups are saved. Empty means the app's own folder.",
    },
    second: { label: "Second folder", tip: "Recommended: a USB drive. Example: E:\\Backups" },
    choose: { label: "Choose…", tip: "Pick a folder with the computer's folder picker." },
    pickTitle: "Choose a folder for backups",
    interval: {
      label: "Every (minutes)",
      tip: "How often to back up while the app is open. Example: 60",
    },
    keep: { label: "Backups to keep", tip: "How many recent backups to keep. Example: 50" },
    numberInvalid: "Type a whole number. Example: 60",
  },
  logs: {
    detailed: {
      label: "Detailed logging",
      tip: "Write extra detail to the log files, to help find a problem. Errors and warnings are always written.",
    },
    note: "Log files are kept for 14 days in the app's logs folder.",
    switched: "Saved. The change works right away.",
  },
};

export type SettingsText = typeof en;
