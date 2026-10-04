---
tags: [runbook, hardware]
---
# Printer Setup

## Receipt printer
| Connection | Steps |
|---|---|
| **USB (most common)** | Install the manufacturer's Windows driver → it appears in Windows "Printers & scanners" → in Settings → Hardware choose **Windows printer** and pick it. The app sends raw ESC/POS through the driver |
| **Network (LAN)** | Give the printer a fixed IP (printer's self-test page shows it) → choose **Network**, enter IP, port 9100 |
| **Serial / USB-serial** | Choose **Serial**, pick the COM port, baud (usually 9600 or 115200) |
| **Mac / Linux, USB** | Many receipt printers have no Mac driver. Add the printer as a **raw** CUPS queue (no driver) → choose **CUPS**. If that doesn't work, use a network or USB-serial connection ([[ADR-017 Windows first all three OS built]]) |
Then pick the paper width (58/80 mm) and press **Test print**. The test receipt contains text, the currency symbol, the shop logo, a barcode and a QR code; all must be sharp.

## Cash drawer
Plugged into the printer's RJ11 "DK" port. In Settings choose kick pin 2 (most drawers) or 5 → **Open drawer**.

## Label printer
- **A4 sheets:** any normal printer through the system print dialog. Choose the sheet layout matching your label paper (e.g. 3×8, 4×10).
- **Thermal label printer (TSPL, e.g. TSC/Xprinter label models):** install as a Windows printer → choose it as **Label printer** → set the label size in mm → **Test label**, then scan the test barcode.

## Known-good printers
Filled in during M13 (first real-hardware test) and by pilot shops: model, connection, notes.
| Model | Width | Connection | Status | Notes |
|---|---|---|---|---|
| (to be filled in M13) | | | | |

## Troubleshooting
| Symptom | Fix |
|---|---|
| Nothing prints, no error | Printer paused or offline in Windows; open "See what's printing" |
| Prints garbage text | The driver isn't passing raw data. Use the manufacturer's driver, not "Generic / Text Only" |
| Image too wide / cut off | Wrong paper width setting |
| Very slow printing | Serial at a low baud. Increase baud, or use USB/LAN |
| Drawer doesn't open | Try the other kick pin; check the cable is in the DK port |
Use **Print to system printer** on the banner to keep trading while fixing it.
