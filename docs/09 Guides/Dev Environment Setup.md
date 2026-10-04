---
tags: [guide]
---
# Dev Environment Setup

## Prerequisites
| Tool | Windows | macOS | Linux |
|---|---|---|---|
| Rust (via rustup; version pinned in `rust-toolchain.toml`) | ✅ | ✅ | ✅ |
| Build tools | Visual Studio Build Tools (C++ workload) | Xcode Command Line Tools | `build-essential`, `libwebkit2gtk-4.1-dev`, `libssl-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `libudev-dev` (serial) |
| WebView2 | Preinstalled on Windows 11 / recent 10 | — | — |
| Node.js LTS + pnpm (via corepack) | ✅ | ✅ | ✅ |
| cargo tools | `cargo install cargo-nextest tauri-cli --locked` | same | same |
| Fast linker (optional) | `lld` (rust-lld) | default | `mold` or `lld` |

The [Tauri prerequisites guide](https://v2.tauri.app/start/prerequisites/) is the authority if this list goes stale.

## On the Mac (the current development and test machine)
- Install the Xcode Command Line Tools: `xcode-select --install`.
- Demo builds are unsigned for now. The first time, right-click the app → **Open** → **Open** (or allow it in System Settings → Privacy & Security).
- USB-serial adapters (for scales and serial printers) may need the maker's driver (CH340, FTDI or Prolific). Ports show up as `/dev/cu.usbserial-…`.
- Receipt printers: a network printer is easiest. USB printers need a raw CUPS queue ([[Printer Setup]]).

## First run
```bash
pnpm install
```
```bash
cargo xtask gen-types
```
```bash
pnpm tauri dev
```

## Everyday commands
| Goal | Command |
|---|---|
| Run the app (Rust + UI, hot reload for UI) | `pnpm tauri dev` |
| UI only, in a browser with the mock bridge | `pnpm dev:mock` |
| Test one module | `cargo nextest run -p catalog` |
| Test everything (Rust) | `cargo nextest run --workspace` |
| UI tests | `pnpm test` · `pnpm e2e` |
| Lint everything | `cargo clippy --all-targets -- -D warnings && pnpm lint` |
| Regenerate TS types | `cargo xtask gen-types` |
| Check module boundaries | `cargo xtask check-deps` |
| Demo data database | `cargo xtask seed --size demo --out dev.sqlite` |
| Benchmarks | `cargo xtask bench` |
| Release build (local) | `pnpm tauri build` |

## Working without hardware (the normal way for now)
No printer, scanner or scale is needed to build or test the app.
| Device | Stand-in | How to use it |
|---|---|---|
| Receipt printer | **Preview printer** | Settings → Hardware → Printer: *Preview*. Every receipt opens in a preview window as the exact picture that would print, and is saved as a PNG in `dev-prints/` |
| Normal printer / PDF | The Mac print dialog | "Print on another printer" → **Save as PDF** |
| Barcode scanner | **Scan simulator** (dev builds only, `Ctrl+Shift+S`) | Click a product's barcode, or type one, and it's "scanned" at scanner speed. Typing a code + Enter in the scan box also works, because real scanners act like a keyboard |
| Scale | **Fake scale** (dev builds only) | A small panel to set the weight and switch it between *stable* and *moving* |
| Cash drawer | Preview printer | The preview window shows "Drawer opened" when a drawer pulse would be sent |
With real devices later: see [[Printer Setup]] and [[Scale Setup]].
