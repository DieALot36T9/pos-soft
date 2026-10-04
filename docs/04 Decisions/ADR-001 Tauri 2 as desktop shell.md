---
tags: [adr]
status: accepted
---
# ADR-001 Tauri 2 as desktop shell

## Context
We need a desktop app for Windows (main target), Linux and macOS that works offline, starts fast, uses little memory on shop PCs, and can talk to serial ports and raw printers. The product owner asked for Tauri.

## Decision
Use **Tauri 2**. The core logic is in Rust and the UI is a web app in the system webview (WebView2 on Windows).

## Consequences
- ✅ Installers of about 10 MB, low memory, fast startup, native access to serial ports and printers through Rust.
- ✅ The Rust core can be tested without any UI.
- ⚠️ Rendering differs slightly between webviews (WebView2 / WebKitGTK / WKWebView). Windows is the reference, and the others are smoke-tested in CI.
- ⚠️ Windows needs WebView2. The installer bundles the bootstrapper.

## Alternatives
- **Electron:** one language (JS) and consistent rendering, but ~150 MB installers, 300+ MB RAM, and native hardware access through Node add-ons.
- **Pure Rust UI (egui/Slint/Iced):** fast, but slow to build polished forms and tables, and weaker complex-script text input.
- **.NET/WPF:** Windows only.
