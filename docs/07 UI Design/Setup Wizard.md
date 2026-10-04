---
tags: [ui]
---
# Setup Wizard

What the shop owner sees the first time the app opens. Requirements: [[Req - Setup and Settings]]. Each step is saved when you press **Next**, so setup can be continued later (SET-02). A list of steps on the left shows where you are. **Every field has a tooltip with an example** ([[Design System#Tooltips]]).

**Welcome screen:** two choices: 🟩 **Set up a new shop** or 🟦 **Restore from a backup** (for moving to a new PC; see [[Installation#Moving to a new PC]]). Restoring checks the file, restores it and goes straight to login (only the hardware step is shown again).

| # | Step | Fields | Required |
|---|---|---|---|
| 1 | Your shop | Name, address, phone, email, tax registration label + number, logo, receipt footer | Name |
| 2 | Money | Currency (searchable list fills in the symbol and number of decimals), symbol position, decimals, separators, digit grouping, **cash rounding** (step, direction, applies to). A live preview shows how `1234567.5` will look | ✅ |
| 3 | Tax | Tax name (VAT, GST, …), **prices include / exclude tax** (with a worked example), list of rates (name + %), optional parts of a rate, default rate | At least one rate |
| 4 | Admin account | Username, display name, password ×2 → **recovery code** screen (Print / I've saved it) | ✅ |
| 5 | This till | Till code, business-day cutoff hour (with an example: "a sale at 01:30 counts for yesterday") | ✅ |
| 6 | Hardware | Receipt printer (connection type, printer/port, paper width) + **Test print**; drawer + **Open drawer**; scale port + type + **Read scale**; label printer | Can skip |
| 7 | Backups | Main folder, second folder (recommended: a USB drive), how often, how many to keep | ✅ (defaults are fine) |
| 8 | Products | Import from CSV/Excel · Add later · Load demo data | Choose one |
| ✓ | Done | Summary, "Create cashier users now" link, 🟩 **Start** | — |

Rules
- Steps 1–5 and 7 are required to finish. Step 6 can be done later; until then, the dashboard shows an amber "Printer not set up" card.
- Settings that lock after the first sale (number of currency decimals) and the warning about changing the tax mode are explained right on the step.
- Bottom bar on every step: ⬜ **Back** (bottom-left), 🟩 **Next** (bottom-right).
