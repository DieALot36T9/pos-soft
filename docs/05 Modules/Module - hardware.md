---
tags: [module]
crate: hardware
milestone: M1 (test), M7
---
# Module: hardware

**Purpose:** talk to devices. Send bytes to printers, read weights from scales, list available printers and serial ports. **No business knowledge.**

**Requirements:** [[Req - Hardware and Printing]]. **Runbooks:** [[Printer Setup]], [[Scale Setup]].
**Depends on:** `core`, `escpos` (its network, USB, serial and file connections), `serialport` (scales), `windows` (winspool, only if the `escpos` drivers can't reach spooler-installed printers; see [[Reuse Map]]). `unsafe` is allowed **only** in `printer/windows_spooler.rs`.

## Public API
```rust
pub trait RawPrinter: Send + Sync { fn send(&self, bytes: &[u8]) -> Result<(), HwError>; fn status(&self) -> PrinterStatus; }
pub enum PrinterConfig {
  WindowsSpooler { printer_name: String },          // OpenPrinter / StartDocPrinter("RAW") / WritePrinter
  Tcp { host: String, port: u16 },                   // 9100, 3 s timeout
  Serial { port: String, baud: u32 },
  Cups { queue: String },                            // `lp -d <queue> -o raw` on Linux/macOS
  Preview,                                          // no printer: shows the receipt picture in a window + saves a PNG
  File { path: PathBuf },                            // development: writes .bin
}
pub fn printer_from(cfg: &PrinterConfig) -> Box<dyn RawPrinter>;
pub fn list_printers() -> Vec<String>;               // OS-installed printers
pub fn list_serial_ports() -> Vec<SerialPortInfo>;

pub trait Scale: Send + Sync { fn read(&self, timeout: Duration) -> Result<Reading, HwError>; }
pub struct Reading { pub value_milli: i64, pub unit: WeightUnit /*g, kg, lb*/, pub stable: bool, pub raw: String }
pub struct ScaleProfile { pub mode: ScaleMode /*Continuous | Request{cmd: Vec<u8>}*/, pub pattern: String /*regex for number+unit*/,
                          pub stable_marker: Option<String>, pub stable_repeats: u8 }
pub fn scale_from(port: &SerialSettings, profile: &ScaleProfile) -> Box<dyn Scale>;
pub fn builtin_scale_profiles() -> Vec<(String, ScaleProfile)>;  // generic continuous, CAS-type, Toledo-type

pub trait FiscalDevice { /* extension point only; no implementations in v1.0 */ }

// fakes, for tests and for working without hardware
pub struct MockPrinter { pub sent: Mutex<Vec<Vec<u8>>> }   pub struct MockScale { pub readings: Vec<Reading> }
pub struct FakeScale { /* weight + stable flag, set from the dev "Fake scale" panel */ }
```
Device calls run on a dedicated hardware thread in `app`, so a slow printer never blocks a command.

**On macOS** (the current test machine), serial ports are `/dev/cu.*`. USB printers without a Mac driver use a raw CUPS queue. If that fails when a printer is first tested (M13), a direct-USB connection is added ([[ADR-017 Windows first all three OS built]]).

## Tests
- Scale parsing for each built-in profile from recorded raw lines (fixtures), including unstable then stable readings.
- TCP printer against a local test listener (bytes received match).
- Windows spooler: manual test checklist in M13 (first real hardware) plus an ignored-by-default test that prints to the "Microsoft Print to PDF" queue (raw is rejected → error path tested).

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/hardware/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/hardware/` |
| **Screen folder** | `ui/src/modules/hardware/` (hardware settings pages, the dev-only scan simulator and fake scale panel) |
| **Requirements** | [[Req - Hardware and Printing]] HW-01 to HW-10 |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | Bytes to send and device settings |
| **Used by** (re-run their tests when the public API changes) | The hardware thread in `app` (printing, drawer, scale) |
| **Test it on its own** | `cargo nextest run -p hardware` · screens: `pnpm test src/modules/<folder>` |

**Watch out for**
- No business knowledge: it doesn't know what a sale or receipt is.
- `unsafe` is allowed only in `printer/windows_spooler.rs`.
- Every device call has a timeout.
- Fakes must behave like real devices (delays, errors), so error handling gets tested without hardware.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- Not started (see [[Task Index]]).
