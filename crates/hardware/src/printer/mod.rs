//! Printer connections. Each one takes ready-made bytes and sends them to a printer.
//! What the bytes mean (a receipt, a label) is the `printing` module's job.

mod cups;
mod file;
mod mock;
mod network;
mod preview;
mod serial;
#[cfg(windows)]
mod windows_spooler;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{Device, HwError};

pub use mock::MockPrinter;
pub use preview::{PreviewInbox, PreviewJob, read_escpos_job};

/// Something that prints raw bytes (ESC/POS for receipts, TSPL for labels).
pub trait RawPrinter: Send + Sync {
    /// Sends the bytes to the printer. Waits at most a few seconds (see each connection).
    fn send(&self, bytes: &[u8]) -> Result<(), HwError>;
    /// Says whether the printer can be reached right now, without printing anything.
    fn status(&self) -> PrinterStatus;
}

/// Whether a printer can be reached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum PrinterStatus {
    /// The printer answered, or the connection is ready to use.
    Ready,
    /// The printer can't be reached. `detail` says why, for the log and for troubleshooting.
    Offline { detail: String },
    /// This kind of connection can't tell without printing.
    Unknown,
}

/// How the shop's printer is connected. Saved by the settings module, passed in by `app`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum PrinterConfig {
    /// A printer installed in Windows. Raw bytes go through the Windows print spooler.
    WindowsSpooler { printer_name: String },
    /// A network printer, e.g. host "192.168.1.50", port 9100.
    Tcp { host: String, port: u16 },
    /// A serial or USB-serial printer, e.g. port "COM3" or "/dev/cu.usbserial-1", baud 9600.
    Serial { port: String, baud: u32 },
    /// A raw CUPS queue on Linux or macOS, printed with `lp -d <queue> -o raw`.
    Cups { queue: String },
    /// No printer: the bytes are turned back into pictures for a preview window (rule HW-10).
    Preview,
    /// For development: every job is added to the end of this file.
    File { path: PathBuf },
}

/// Makes the printer connection for these settings.
/// `preview` collects the jobs of a `Preview` printer; `app` keeps one and shows its jobs.
/// Nothing is opened yet: every `send` opens the connection again, so a printer that was
/// switched off and on again works without restarting the app.
pub fn printer_from(config: &PrinterConfig, preview: &PreviewInbox) -> Box<dyn RawPrinter> {
    match config {
        PrinterConfig::WindowsSpooler { printer_name } => Box::new(SpoolerPrinter {
            printer_name: printer_name.clone(),
        }),
        PrinterConfig::Tcp { host, port } => {
            Box::new(network::NetworkPrinter::new(host.clone(), *port))
        }
        PrinterConfig::Serial { port, baud } => {
            Box::new(serial::SerialPrinter::new(port.clone(), *baud))
        }
        PrinterConfig::Cups { queue } => Box::new(cups::CupsPrinter::new(queue.clone())),
        PrinterConfig::Preview => Box::new(preview::PreviewPrinter::new(preview.clone())),
        PrinterConfig::File { path } => Box::new(file::FilePrinter::new(path.clone())),
    }
}

/// The names of the printers installed on this computer.
/// Windows: the printers in "Printers & scanners". Linux and macOS: the CUPS queues.
/// Returns an empty list if the computer can't tell.
pub fn list_printers() -> Vec<String> {
    #[cfg(windows)]
    let found = windows_spooler::installed_printers();
    #[cfg(not(windows))]
    let found = cups::installed_queues();
    found.unwrap_or_default()
}

/// A printer installed in Windows. On other systems it always fails with `NotConnected`.
struct SpoolerPrinter {
    printer_name: String,
}

impl RawPrinter for SpoolerPrinter {
    fn send(&self, bytes: &[u8]) -> Result<(), HwError> {
        #[cfg(windows)]
        return windows_spooler::send(&self.printer_name, bytes);
        #[cfg(not(windows))]
        {
            let _ = bytes;
            Err(only_on_windows(&self.printer_name))
        }
    }

    fn status(&self) -> PrinterStatus {
        #[cfg(windows)]
        return windows_spooler::status(&self.printer_name);
        #[cfg(not(windows))]
        PrinterStatus::Offline {
            detail: only_on_windows(&self.printer_name).to_string(),
        }
    }
}

#[cfg(not(windows))]
fn only_on_windows(printer_name: &str) -> HwError {
    HwError::NotConnected {
        device: Device::Printer,
        detail: format!("\"{printer_name}\" is a Windows printer; this computer isn't Windows"),
    }
}

/// Turns an error from the `escpos` crate into ours.
fn escpos_error(error: escpos::errors::PrinterError, connecting: bool) -> HwError {
    let detail = error.to_string();
    if connecting {
        HwError::NotConnected {
            device: Device::Printer,
            detail,
        }
    } else {
        HwError::SendFailed {
            device: Device::Printer,
            detail,
        }
    }
}
