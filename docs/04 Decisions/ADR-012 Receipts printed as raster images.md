---
tags: [adr]
status: accepted (confirm on a real printer in M13)
---
# ADR-012 Receipts printed as images

## Context
Receipt printers have a built-in "text mode", but its character sets differ by model, and most can't print symbols such as ৳ or ₹. The app is universal, so any currency symbol may be needed. Shops also want their logo on the receipt.

## Decision
- The `printing` crate **draws every receipt, report slip, label and shelf tag as a black-and-white picture** at the printer's width (384 dots for 58 mm paper, 576 dots for 80 mm).
- Text is drawn with **cosmic-text** and **tiny-skia** using one bundled open-licence font (**Noto Sans**).
- The picture is sent with the standard ESC/POS image command (`GS v 0`) in strips, then the paper is cut and the drawer opened if needed.
- Label printers get the same picture using their own language (TSPL `BITMAP`).
- If the receipt printer fails, the same receipt can be printed on any normal printer through the system print dialog ([[ADR-013 A4 and PDF output through the webview]]).

## Consequences
- ✅ One way of printing that works on every ESC/POS printer: any currency symbol and the logo always print correctly.
- ✅ The preview on screen is the exact picture that prints, and tests compare pictures.
- ⚠️ A picture is more data than text (about 20–60 KB per receipt). Over USB or network this takes well under a second. Measured on a real printer in M13 (no printer is available yet).

## Alternative
Printer text mode: a little faster, but symbols and logos vary by printer model. Rejected for reliability.
