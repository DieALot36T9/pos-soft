---
tags: [module]
crate: hardware
milestone: M1 (test), M7
---
# Module: hardware

**Purpose:** talk to devices. Send bytes to printers, read weights from scales, list available printers and serial ports. **No business knowledge.**

**Requirements:** [[Req - Hardware and Printing]]. **Runbooks:** [[Printer Setup]], [[Scale Setup]].
**Depends on:** `core`, `serde`, `thiserror`, `ts-rs`, `escpos` (its network, serial and file drivers; default features off, `std` + `serial_port` on), `serialport` (scales, serial printers, port list), `regex` (scale profile patterns), `windows` (winspool; Windows builds only, because the `escpos` crate has no spooler driver; see [[Reuse Map]]). `unsafe` is allowed **only** in `printer/windows_spooler.rs`.

**Tables:** none. Device settings are saved by the settings module; `app` passes them in.

## Public API
```rust
// printer/
pub trait RawPrinter: Send + Sync { fn send(&self, bytes: &[u8]) -> Result<(), HwError>; fn status(&self) -> PrinterStatus; }
pub enum PrinterStatus { Ready, Offline { detail: String }, Unknown }   // Unknown: the connection can't tell without printing
pub enum PrinterConfig {                             // serde: {"kind": "tcp", "host": …, "port": …}; stored by settings
  WindowsSpooler { printer_name: String },           // OpenPrinter / StartDocPrinter("RAW") / WritePrinter; 10 s limit
  Tcp { host: String, port: u16 },                   // 9100; IP or host name; 3 s limit to connect and per write
  Serial { port: String, baud: u32 },                // 5 s limit per write
  Cups { queue: String },                            // `lp -d <queue> -o raw` on Linux/macOS; 10 s limit, then lp is stopped
  Preview,                                           // no printer: the jobs go to the PreviewInbox (below)
  File { path: PathBuf },                            // development: each job is added to the end of the file; 5 s limit
}
pub fn printer_from(cfg: &PrinterConfig, preview: &PreviewInbox) -> Box<dyn RawPrinter>; // opens nothing; each send connects again
pub fn list_printers() -> Vec<String>;               // Windows: installed printers (EnumPrinters). Linux/macOS: CUPS queues (`lpstat -e`)
pub fn list_serial_ports() -> Vec<SerialPortInfo>;   // sorted; on macOS only /dev/cu.*
pub struct SerialPortInfo { pub name: String, pub kind: PortKind /*Usb|Bluetooth|Pci|Other*/, pub product: Option<String> }

// Preview printer (HW-10): reads the ESC/POS bytes like a printer would
pub struct PreviewInbox;                             // app keeps one; clones share the jobs. new(), take_jobs() -> Vec<PreviewJob>
pub struct PreviewJob { pub pictures: Vec<Bitmap1>, pub drawer_opened: bool, pub bytes: Vec<u8> }
pub fn read_escpos_job(bytes: &[u8]) -> PreviewJob;  // `GS v 0` picture bands (same width → one picture) and `ESC p` drawer pulses

// scale/
pub trait Scale: Send + Sync { fn read(&self, timeout: Duration) -> Result<Reading, HwError>; }
//   waits for a stable weight; at the time limit returns the last weight with stable: false;
//   no weight at all → Timeout, or NoWeight { raw } if data came without a weight in it
pub struct Reading { pub value_milli: i64, pub unit: WeightUnit /*G | Kg | Lb*/, pub stable: bool, pub raw: String }
pub struct ScaleProfile { pub mode: ScaleMode /*Continuous | Request { command: Vec<u8> }*/,
                          pub pattern: String,       // regex with a `value` group and an optional `unit` group
                          pub default_unit: WeightUnit, // when the line has no unit (e.g. Toledo)
                          pub stable_marker: Option<String>, pub stable_repeats: u8 }
pub struct SerialSettings { pub port: String, pub baud: u32, pub data_bits: DataBits /*Seven|Eight*/,
                            pub parity: Parity /*None|Odd|Even*/, pub stop_bits: StopBits /*One|Two*/ }
pub fn scale_from(port: &SerialSettings, profile: &ScaleProfile) -> Box<dyn Scale>; // opens the port on each read
pub fn check_profile(profile: &ScaleProfile) -> Result<(), HwError>;               // for the settings screen, before saving
pub fn builtin_scale_profiles() -> Vec<(String, ScaleProfile)>;  // ids "generic_continuous", "cas", "toledo"; the screen names them

pub trait FiscalDevice: Send + Sync {}               // HW-09 extension point only; no methods, no implementations in v1.0

// errors: one enum, turns into AppError::Hardware { device: "printer"|"scale", code, detail }
pub enum Device { Printer, Scale }
pub enum HwError { Timeout { device, waited_ms }, NotConnected { device, detail }, SendFailed { device, detail },
                   Refused { device, detail }, NoWeight { raw }, BadSettings { device, detail } }
//   codes: timeout, not_connected, send_failed, refused, no_weight, bad_settings

// fakes, for tests and for working without hardware. They are slow and fail like real devices.
pub struct MockPrinter { pub sent: Mutex<Vec<Vec<u8>>>, … }  // new(), with_delay(d), fail_next(err), set_status(s), sent_jobs()
pub struct MockScale;                                // new(Vec<Result<Reading, HwError>>), with_delay(d); then Timeout when used up
pub struct FakeScale;                                // the dev "Fake scale" panel: set_weight, set_stable, set_connected, state();
                                                     // clones share the state; a new line every 100 ms like a real scale
```
Types the UI sees derive `ts_rs::TS`: `PrinterConfig`, `PrinterStatus`, `SerialPortInfo`, `PortKind`, `Reading`, `WeightUnit`, `ScaleProfile`, `ScaleMode`, `SerialSettings`, `DataBits`, `Parity`, `StopBits`.

Device calls run on a dedicated hardware thread in `app`, so a slow printer never blocks a command.

**Preview printer:** the `hardware` crate turns the bytes back into pictures (`PreviewJob`). `app` shows them in the preview window and saves them as PNG files with `printing::to_png`, so `hardware` needs no image library and never depends on `printing`.

**Built-in scale profiles** (vendor protocols, [[Reuse Map]]):
| Id | Mode | What the scale sends | Stable when |
|---|---|---|---|
| `generic_continuous` | Continuous | `  1.234 kg` | 3 equal readings in a row (HW-05) |
| `cas` | Continuous | `ST,GS,+  1.234kg` (`US` = unstable, `OL` = overload) | the line has `ST,` |
| `toledo` | Request `W` (Mettler-Toledo 8217) | STX `01.234` CR; STX `?` status CR while moving | any weight answer (it only answers a weight when settled) |

**On macOS** (the current test machine), serial ports are `/dev/cu.*`. USB printers without a Mac driver use a raw CUPS queue. If that fails when a printer is first tested (M13), a direct-USB connection is added ([[ADR-017 Windows first all three OS built]]).

## Tests
Test names start with the requirement ID (`hw_02_…`, `hw_05_…`, `hw_08_…`, `hw_10_…`).
- `src/scale/fixture_tests.rs`: each built-in profile reads its lines from `tests/fixtures/` through the real read loop: unstable then stable readings, a weight that never settles, data with no weight (`NoWeight` with the raw text), a silent scale (`Timeout`), and a request scale asked again when it doesn't answer.
- `src/scale/parse.rs`: numbers to thousandths without floats, units, stable marker, "3 equal readings", bad profiles refused, line splitting.
- `src/printer/preview.rs`: `GS v 0` bands become pictures (joined when the width matches), drawer pulse noticed, cut-off pictures dropped, inbox hands out each job once.
- `tests/tcp_printer.rs`: network printer against a local listener (bytes received match exactly), host names, a printer that's off fails quickly, status.
- `tests/contract.rs`: every public function and fake as described above, the JSON shape of `PrinterConfig` and `ScaleProfile` (settings stores it), errors → `AppError::Hardware` codes.
- `tests/windows_spooler.rs` (Windows only): an unknown printer is `NotConnected`; an ignored-by-default test prints to the "Microsoft Print to PDF" queue (raw is rejected → error path tested). Run it by hand: `cargo nextest run -p hardware --run-ignored only`. Manual checklist with a real printer in M13.
- `src/wait.rs`: time limits on threads and on programs (`lp`) that hang.

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/hardware/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/hardware/` |
| **Screen folder** | `ui/src/modules/hardware/` (hardware settings pages, the dev-only scan simulator and fake scale panel) |
| **Requirements** | [[Req - Hardware and Printing]] HW-01 to HW-10 |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | Bytes to send, device settings (`PrinterConfig`, `SerialSettings`, `ScaleProfile`) and the `PreviewInbox` |
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
- Built in task B-hardware. **Connected in C-M1:** `app` keeps one `PreviewInbox` and one `FakeScale` (`app::devices::Devices`); "Test print" goes to the Preview printer, whose printouts the screen shows. Screens in `ui/src/modules/hardware/`: test print (58/80 mm), the Preview printer panel (pictures, "Drawer opened", "Print on another printer"), and, in dev builds, the fake scale panel, a scan test box and the scan simulator (`Ctrl+Shift+S`; sample codes in `components/sampleCodes.ts`). Tests: `HardwarePage.test.tsx` and `ui/e2e/hardware.spec.ts`. **Connected in C-M7:** `app` builds the receipt printer from the saved `PrinterConnection` with `printer_from` (Preview until one is chosen), sends receipts, reports and `ESC p` drawer pulses through it, and reads the saved scale with `scale_from` and a built-in profile (8N1 for now); Settings → Printer and scale chooses them. The dev fake scale is used only when no scale is saved.
- **Real hardware not tried yet (M13):** the Windows spooler code is built and checked by clippy in the Windows CI job, and its "unknown printer" test runs there, but it has never printed to a real printer. Run the ignored "Microsoft Print to PDF" test and the M13 checklist on a Windows PC. Same for CUPS on the Mac and for real scales.
- **Scale fixtures** in `tests/fixtures/` follow each vendor's published format. Replace them with recordings from real scales in M13 and keep the tests passing.
- **Printer status** is basic: network = can connect; CUPS = `lpstat -p`; Windows = the printer exists; serial = the port exists (`Unknown`). Paper-out and cover-open (ESC/POS `DLE EOT`) are not read.
- **Preview** reads only `GS v 0` pictures (what the `escpos` crate and the planned `printing` encoder send) and ignores the size mode byte. If `printing` ever sends `GS ( L` pictures, add them to `read_escpos_job`.
- **Network printers:** a host name is looked up by the computer's own resolver, which has its own time limit. Printers are normally set up with a fixed IP address ([[Printer Setup]]).
- **UI error text:** done in C-M1: `ui/src/shared/text/en/errors.ts` has a message per code (`hardware_timeout`, `hardware_not_connected`, `hardware_send_failed`, `hardware_refused`, `hardware_no_weight`, `hardware_bad_settings`); an unknown code falls back to the general message.
- **Labels (HW-06)** and the system print dialog (HW-07) are not part of this crate: TSPL bytes go through any `RawPrinter`; the dialog is `app`'s job.
