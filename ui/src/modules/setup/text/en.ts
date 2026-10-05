// All screen text for the setup wizard, in plain English (ADR-022).
export const en = {
  welcome: {
    title: "Welcome to Supershop POS",
    intro:
      "Let's get your shop ready. It takes about ten minutes, and you can stop and carry on later.",
    newShop: {
      label: "Set up a new shop",
      tip: "Start with an empty shop and answer a few questions.",
    },
    restore: {
      label: "Restore from a backup",
      tip: "Moving to a new computer? Pick a backup file from the old one.",
    },
    pickTitle: "Choose a backup file",
    restoring: "Restoring the backup. The app will restart…",
  },
  steps: {
    shop: "Your shop",
    currency: "Money",
    tax: "Tax",
    admin: "Admin account",
    till: "This till",
    hardware: "Printer",
    backup: "Backups",
    products: "Products",
    done: "Done",
    listLabel: "Setup steps",
    numbered: "{{number}}. {{name}}",
    doneMark: "Done",
    current: "You are here",
  },
  nav: {
    back: { label: "Back", tip: "Go to the step before. What you saved stays saved." },
    next: { label: "Next", tip: "Save this step and go to the next one." },
    skip: {
      label: "Skip for now",
      tip: "Set this up later in Settings. The app works without it.",
    },
    saving: "Saving…",
  },
  shop: {
    title: "Your shop",
    intro: "This is printed at the top of every receipt.",
    name: { label: "Shop name", tip: "The name customers know. Example: Rahim Store" },
    address: { label: "Address", tip: "One line of the address. Example: 12 Lake Road, Dhaka" },
    phone: { label: "Phone", tip: "The shop's phone number. Example: 01711-000000" },
    email: { label: "Email", tip: "Optional. Example: shop@example.com" },
    taxRegLabel: {
      label: "Tax number name",
      tip: "What your tax number is called. Example: BIN, VAT No, GSTIN",
    },
    taxRegNo: { label: "Tax number", tip: "Your tax registration number. Example: 000123456-0101" },
    footer: {
      label: "Receipt footer",
      tip: "A line at the bottom of receipts. Example: Thank you!",
    },
  },
  money: {
    title: "Money",
    intro: "How amounts are written, and how cash totals are rounded.",
    currency: { label: "Currency", tip: "Pick your currency. It fills in the sign and decimals." },
    symbol: { label: "Currency sign", tip: "The sign printed with amounts. Example: ৳" },
    symbolBefore: { label: "Sign before the amount", tip: "Tick for ৳100, untick for 100 ৳." },
    digits: {
      label: "Decimals",
      tip: "Digits after the point. Can't change after the first sale.",
    },
    grouping: {
      label: "Digit grouping",
      tip: "How big numbers are split. Indian: 12,34,567. Western: 1,234,567.",
      indian: "Indian (12,34,567)",
      western: "Western (1,234,567)",
    },
    preview: "1234567.5 is written as",
    rounding: {
      heading: "Cash rounding",
      step: {
        label: "Round to",
        tip: "Round totals to this amount. Example: 1.00 rounds 123.45 to 123.00. Use 0 for no rounding.",
      },
      mode: {
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
    lockNote: "The number of decimals can't change after the first sale.",
  },
  tax: {
    title: "Tax",
    intro: "Each product gets one rate. You can add more rates later.",
    label: { label: "Tax name", tip: "What your tax is called. Example: VAT, GST, Sales Tax" },
    mode: {
      label: "Shelf prices",
      tip: "Do the prices on the shelf already include tax?",
      inclusive: "Include tax",
      exclusive: "Don't include tax",
    },
    example: {
      inclusive:
        "Example: a price of 115.00 with 15% tax is 100.00 + 15.00 tax. The customer pays 115.00.",
      exclusive:
        "Example: a price of 100.00 with 15% tax gets 15.00 added. The customer pays 115.00.",
    },
    modeNote: "Changing this after sales affects only future sales.",
    ratesHeading: "Tax rates",
    noRates: "No rates yet. Add one below.",
    rateName: { label: "Rate name", tip: "A name for the rate. Example: Standard 15%" },
    ratePercent: { label: "Percent", tip: "The rate, up to 2 decimals. Example: 7.5" },
    percentInvalid: "Type a percentage from 0 to 100, with at most 2 decimals. Example: 7.5",
    addRate: { label: "Add rate", tip: "Add this rate to the list." },
    defaultRate: {
      label: "Default rate",
      tip: "The rate new products get unless you pick another.",
    },
  },
  admin: {
    title: "Admin account",
    intro: "The admin can change settings, add staff and approve refunds.",
    username: { label: "Username", tip: "The name you sign in with. Example: owner" },
    displayName: { label: "Your name", tip: "Shown on screen. Example: Rahim Uddin" },
    password: { label: "Password", tip: "At least 8 characters. Keep it secret." },
    passwordAgain: { label: "Password again", tip: "Type the same password once more." },
    codeTitle: "Your recovery code",
    codeIntro:
      "If you forget the admin password, this code lets you set a new one. It is shown only now. Write it down and keep it safe.",
    print: { label: "Print", tip: "Print the code on a normal printer to keep it." },
    saved: { label: "I have saved it", tip: "Go on to the next step." },
    alreadyDone: "The admin account is made. Change it later under Users.",
  },
  till: {
    title: "This till",
    intro: "Each till has a short code, used in invoice numbers like T1-000123.",
    code: { label: "Till code", tip: "1 to 6 letters or digits. Example: T1" },
    cutoff: {
      label: "New day starts at",
      tip: "Sales before this hour count for the day before. Example: with 4, a sale at 01:30 counts for yesterday.",
    },
    hour: "{{hour}}:00",
  },
  hardware: {
    title: "Printer",
    intro: "Pick the receipt printer. Without one, the Preview printer shows receipts on screen.",
    printer: {
      label: "Receipt printer",
      tip: "How the receipt printer is connected.",
      preview: "Preview printer (no printer)",
      none: "No printer yet",
    },
    paper: {
      label: "Paper width",
      tip: "The width of the receipt paper roll.",
      mm58: "58 mm",
      mm80: "80 mm",
    },
    drawer: {
      label: "Cash drawer pin",
      tip: "Most drawers use pin 2. Try pin 5 if the drawer doesn't open.",
      pin2: "Pin 2",
      pin5: "Pin 5",
    },
    later: "Network, USB and serial printers are set up in Settings once you have one.",
  },
  backup: {
    title: "Backups",
    intro: "The app saves a copy of your data regularly and when it closes. The defaults are fine.",
    folder: {
      label: "Backup folder",
      tip: "Where backups are saved. Leave empty to use the app's own folder.",
    },
    second: {
      label: "Second folder",
      tip: "Recommended: a USB drive, so a broken computer doesn't take the backups with it.",
    },
    choose: { label: "Choose…", tip: "Pick a folder with the computer's folder picker." },
    pickTitle: "Choose a folder for backups",
    interval: {
      label: "Every (minutes)",
      tip: "How often to back up while the app is open. Example: 60",
    },
    keep: { label: "Backups to keep", tip: "How many recent backups to keep. Example: 50" },
    numberInvalid: "Type a whole number. Example: 60",
  },
  products: {
    title: "Products",
    intro: "You can import products from a spreadsheet later, or add them one by one.",
    later: "Add products later",
    laterNote: "Importing from CSV or Excel and demo data come in a later version.",
  },
  done: {
    title: "Your shop is ready",
    intro: "Setup is finished. You can change any of this in Settings.",
    printerNote: "The printer isn't set up yet. You can do it in Settings.",
    start: { label: "Start", tip: "Open the app." },
  },
};

export type SetupText = typeof en;
