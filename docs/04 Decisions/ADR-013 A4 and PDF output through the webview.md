---
tags: [adr]
status: accepted
---
# ADR-013 A4 and PDF output through the webview

## Context
Reports, POs, statements, A4 label sheets and fallback receipts need A4 or normal-printer output and PDF export.

## Decision
- Render the document as **HTML/CSS in a print-only React view** from data Rust provides, and call the webview's print, which opens the system print dialog. Users choose a printer or **"Save as PDF"** (built into Windows 10+ and macOS; Linux via CUPS-PDF/GTK).
- Barcodes on A4 labels are SVGs generated in Rust (`barcoders`).

## Consequences
- ✅ The browser engine shapes text perfectly, and there's no PDF library to maintain.
- ✅ Print CSS (`@page`) gives exact A4 layouts.
- ⚠️ PDF export always goes through the print dialog (one extra click). Acceptable.
- ⚠️ Print CSS must be tested on WebView2 specifically (the reference).
