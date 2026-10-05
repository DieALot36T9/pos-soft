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
| cargo tools | `cargo install cargo-nextest cargo-deny tauri-cli --locked` | same | same |
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
| Check module boundaries, licences and advisories | `cargo deny check` |
| Prove the module bans work | `bash scripts/check-module-bans.sh` (macOS/Linux) |
| Demo data database | `cargo xtask seed --size demo --out dev.sqlite` |
| Benchmarks | `cargo xtask bench` |
| Release build (local) | `pnpm tauri build` |

## Working without hardware (the normal way for now)
No printer, scanner or scale is needed to build or test the app.
| Device | Stand-in | How to use it |
|---|---|---|
| Receipt printer | **Preview printer** | Settings → Hardware → Printer: *Preview* (for now the only printer: the "Printer and devices" page on the first screen). Every printout shows in the Preview printer panel as the exact picture that would print, and is saved as a PNG in `dev-prints/` inside the app's data folder (on the Mac: `~/Library/Application Support/com.supershop.pos/dev-prints/`). The panel shows each file's path |
| Normal printer / PDF | The Mac print dialog | "Print on another printer" → **Save as PDF** |
| Barcode scanner | **Scan simulator** (dev builds only, `Ctrl+Shift+S`) | Click in a text box, then click a sample code (or type one) and it's "scanned" into that box at scanner speed, followed by Enter. Typing a code + Enter also works, because real scanners act like a keyboard. The "Scan test box" shows what arrived and how long it took |
| Scale | **Fake scale** (dev builds only) | A small panel to set the weight, switch it between *still* and *moving*, unplug it, and "Read scale" the way checkout will |
| Cash drawer | Preview printer | The preview window shows "Drawer opened" when a drawer pulse would be sent |
"Dev builds only" means debug builds (`pnpm tauri dev`) and builds made with the `dev-tools` feature: the Mac `.dmg` made for a pull request has it, a version tag's build does not.
With real devices later: see [[Printer Setup]] and [[Scale Setup]].
