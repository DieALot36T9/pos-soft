---
tags: [requirements]
modules: [printing, hardware]
---
# Requirements: Hardware and Printing (HW)

Modules: [[Module - printing]], [[Module - hardware]]. Runbooks: [[Printer Setup]], [[Scale Setup]].

### HW-01 Barcode scanner
- Any USB/Bluetooth scanner in **keyboard mode** with an Enter suffix works with no setup
- The checkout input keeps focus whenever no dialog is open, so scans are never lost
- Scans typed into a dialog by mistake (fast burst + Enter) are ignored and an error is shown, so they don't trigger the dialog's actions

### HW-02 Receipt printer
- Paper: 58 mm (384 dots) or 80 mm (576 dots)
- Connection types: **Windows installed printer** (raw data through the spooler), **network** (IP:9100), **serial / USB-serial** (port, baud), **CUPS** printer on Linux/macOS
- "Test print" prints a sample receipt with text, the currency symbol, the shop logo, a barcode and a QR code

### HW-03 Any symbol on receipts
- Receipts are drawn as **pictures**, so the shop logo and any currency symbol (৳, ₹, €) print correctly on any ESC/POS printer ([[ADR-012 Receipts printed as raster images]])

### HW-04 Cash drawer
- Opened by a pulse from the printer (pin 2 or pin 5, configurable). It opens after cash sales, cash in/out, no-sale, and refunds in cash

### HW-05 Scale
- Serial or USB-serial. Settings: port, baud, data bits, parity, stop bits, **protocol profile**:
  - *Continuous*: the scale sends weight lines all the time
  - *Request*: the app sends a request command (e.g. `W`, `0x05`) and reads the reply
  - Parsing: a pattern that finds the number and unit, plus a stability flag if the protocol has one. If it doesn't, the weight counts as stable after it stays the same for 3 readings
- Profiles ship for common protocols (generic continuous, CAS-type, Toledo-type). Others can be added in settings
- "Read scale" in settings shows the raw data received, for troubleshooting

### HW-06 Label printing
- A4 label sheets through the system print dialog
- Thermal label printers (TSPL language) with a raster image, sizes in mm

### HW-07 Fallback printing
- Any receipt or report can be printed through the **system print dialog** (normal Windows printer) when the thermal printer fails

### HW-08 Failures never block sales
- Printer, drawer and scale errors are shown and logged, never roll back a saved sale, and always offer Retry

### HW-10 Working without hardware
- With no printer: choosing the **Preview** printer shows each receipt on screen exactly as it would print (and saves it as a picture). The drawer pulse is shown as a message
- With no scanner: typing a barcode and pressing Enter works the same as a scan. Dev builds have a **scan simulator**
- With no scale: manual weight entry always works. Dev builds have a **fake scale** panel
- This is how the app is developed and demoed until real hardware is available ([[Dev Environment Setup#Working without hardware (the normal way for now)]])

### HW-09 Fiscal devices (extension point)
- Not built in v1.0. A `FiscalDevice` trait marks where a country-specific device (e.g. EFD/SDC) would plug in ([[Scope#Out of scope]])
