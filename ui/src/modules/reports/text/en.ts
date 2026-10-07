// All screen text for the Reports screen, in plain English (ADR-022).
// Report titles, column titles and row names come from Rust with the report.
export const en = {
  page: {
    title: "Reports",
    intro: "Pick a report on the left, choose the days, then press Show.",
  },
  common: {
    loading: "Loading…",
    none: "All",
  },
  list: {
    label: "Reports to choose from",
    groups: {
      sales: "Sales",
      profit: "Profit",
      stock: "Stock",
      tax: "Tax",
      money_owed: "Customers and suppliers",
      control: "Control",
    },
  },
  reports: {
    daily_summary: {
      label: "Daily summary",
      tip: "Sales, discounts, returns, tax and payments for one business day.",
    },
    shift: {
      label: "Shift reports (X and Z)",
      tip: "The X report of an open shift or the Z report of a closed one. Past Z reports can be printed again.",
    },
    sales_by: {
      label: "Sales analysis",
      tip: "Sales by product, category, brand, cashier, hour, weekday or day, with filters and top N.",
    },
    profit: {
      label: "Profit and margin",
      tip: "Net sales, cost, gross profit and margin, using the cost saved on each sale.",
    },
    stock_value: {
      label: "Stock value",
      tip: "What the stock on hand is worth at cost, by product, category or brand.",
    },
    low_stock: {
      label: "Low stock",
      tip: "Products below their reorder level, with the supplier and how many to order.",
    },
    near_expiry: {
      label: "Near expiry",
      tip: "Batches that expire soon, and ones that have expired.",
    },
    stock_movement: {
      label: "Stock movement",
      tip: "Opening stock, received, sold, returned, adjusted and closing stock for the days chosen.",
    },
    tax: {
      label: "Tax report",
      tip: "Tax collected and refunded by rate and by part of the rate.",
    },
    payables_aging: {
      label: "What we owe suppliers",
      tip: "Each supplier's balance, split by how many days it is overdue.",
    },
    receivables_aging: {
      label: "What customers owe",
      tip: "Each customer's debt, split by how old it is.",
    },
    supplier_statement: {
      label: "Supplier statement",
      tip: "Every delivery, payment and return for one supplier, with balances.",
    },
    customer_statement: {
      label: "Customer statement",
      tip: "Every credit sale, payment and return for one customer, with balances.",
    },
    discounts_overrides: {
      label: "Discounts and overrides",
      tip: "Per cashier: manual discounts, typed prices, voids, cancelled sales, no-sale openings and approvals.",
    },
  },
  params: {
    preset: {
      label: "Days",
      tip: "Pick a common choice, or Custom to type the first and last day.",
      today: "Today",
      yesterday: "Yesterday",
      this_week: "This week",
      this_month: "This month",
      custom: "Custom",
    },
    date: {
      label: "Business day",
      tip: "The business day to report on. Example: 2026-03-02",
    },
    from: { label: "From", tip: "The first business day, included. Example: 2026-03-01" },
    to: { label: "To", tip: "The last business day, included. Example: 2026-03-31" },
    till: { label: "Till", tip: "Only this till's sales, or All for every till." },
    shift: {
      label: "Shift",
      tip: "The shift to report on. Open shifts give an X report, closed ones a Z report.",
      empty: "No shifts yet.",
      open: "open",
      closed: "closed",
    },
    by: {
      label: "Group by",
      tip: "What each row of the report is.",
      product: "Product",
      category: "Category",
      brand: "Brand",
      cashier: "Cashier",
      hour: "Hour of day",
      weekday: "Day of week",
      date: "Day",
    },
    category: { label: "Category", tip: "Only sales of this category, or All." },
    brand: { label: "Brand", tip: "Only sales of this brand, or All." },
    cashier: { label: "Cashier", tip: "Only this cashier's sales, or All." },
    top: {
      label: "Top",
      tip: "Show only the first rows. Leave empty for all. Example: 10",
    },
    days: {
      label: "Days ahead",
      tip: "Batches that expire within this many days. Example: 30",
    },
    supplier: {
      label: "Supplier",
      tip: "The supplier whose statement to show.",
      pick: "Choose a supplier",
    },
    customerSearch: {
      label: "Find customer",
      tip: "Type part of the name or phone number, then press Find. Example: 01711",
    },
    find: { label: "Find", tip: "Look for customers matching what you typed." },
    customer: {
      label: "Customer",
      tip: "The customer whose statement to show. Find them first.",
      none: "Find a customer first.",
    },
  },
  actions: {
    show: { label: "Show", tip: "Run the report and show it below." },
    print80: {
      label: "Print receipt",
      tip: "Print the report on the receipt printer (80 or 58 mm, as in the settings).",
    },
    printA4: {
      label: "Print A4 or PDF",
      tip: "Open the print dialog for an A4 page. Choose Save as PDF there to make a PDF.",
    },
    excel: {
      label: "Excel",
      tip: "Save the report as an Excel file with real numbers.",
      saveTitle: "Save the report as",
    },
    needReport: "Show the report first.",
  },
  view: {
    period: "Period: {{from}} to {{to}}",
    day: "Day: {{date}}",
    filter: "{{name}}: {{value}}",
    generated: "Generated: {{time}}",
    empty: "Nothing to show for this choice.",
    total: "Total",
  },
  done: {
    printed: "The report was printed.",
    exported: "The report was saved as an Excel file.",
  },
  problems: {
    dateInvalid: "Type the day as year-month-day. Example: 2026-03-02",
    topInvalid: "Type a whole number for Top, or leave it empty. Example: 10",
    daysInvalid: "Type a whole number of days. Example: 30",
    pickShift: "Choose a shift first.",
    pickSupplier: "Choose a supplier first.",
    pickCustomer: "Find and choose a customer first.",
  },
};

export type ReportsText = typeof en;
