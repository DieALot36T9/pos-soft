---
tags: [runbook]
---
# Troubleshooting

| Problem | What to do |
|---|---|
| App won't start | Restart the PC. If it still fails: run the installer again (data is kept). Collect logs from `%APPDATA%\<app-id>\logs` |
| "Database check failed" | [[Backup and Restore#The app says "Database check failed" at startup]] |
| Scanner beeps but nothing appears | Click the checkout screen so the window has focus. Check the scanner adds Enter after codes (scan the scanner's "Add CR suffix" setup code) |
| Scanned code shows "not found" but the product exists | The barcode isn't on the product; add it in Products → Barcodes. Check the scanner isn't adding/removing leading zeros (UPC vs EAN setting) |
| Wrong characters when scanning | Keyboard layout: scanners type as a US keyboard. Set the scanner to the PC's keyboard language, or switch Windows input to English |
| Receipt doesn't print | [[Printer Setup#Troubleshooting]] |
| Scale problems | [[Scale Setup#Troubleshooting]] |
| Forgot admin password | Login screen → "Forgot admin password" → enter the recovery code |
| Cashier locked out | Wait 60 seconds, or an admin resets their PIN |
| Can't close the shift | There's an open or held cart on it. Complete, hold or cancel it |
| Totals look wrong | Check the tax mode (inclusive/exclusive) and rounding settings. Reprint the receipt and check against [[Money Tax and Rounding]]. Report with the invoice number |
| Slow after years of use | Back office → Settings → Logs → Export logs and send them. The database can be checked with "Verify data" |

## Sending a problem report
Settings → Logs → **Export logs** (zip). Include: what you were doing, the time, the invoice number if any, and a photo of the screen.
