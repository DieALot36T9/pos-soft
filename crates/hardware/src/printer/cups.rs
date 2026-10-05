//! A raw CUPS queue on Linux or macOS. Many receipt printers have no Mac driver;
//! a raw queue passes our bytes to the printer unchanged.

use std::time::Duration;

use super::{PrinterStatus, RawPrinter};
use crate::wait::run_program;
use crate::{Device, HwError};

/// `lp` only hands the job to CUPS, so it should finish quickly.
const LP_TIMEOUT: Duration = Duration::from_secs(10);
/// Asking CUPS about its queues.
const LPSTAT_TIMEOUT: Duration = Duration::from_secs(3);

pub(crate) struct CupsPrinter {
    queue: String,
}

impl CupsPrinter {
    pub(crate) fn new(queue: String) -> CupsPrinter {
        CupsPrinter { queue }
    }
}

impl RawPrinter for CupsPrinter {
    fn send(&self, bytes: &[u8]) -> Result<(), HwError> {
        // `-o raw` stops CUPS from changing the bytes.
        let args = ["-d", self.queue.as_str(), "-o", "raw"];
        let output = run_program(Device::Printer, "lp", &args, bytes, LP_TIMEOUT)?;
        if output.success {
            Ok(())
        } else {
            Err(HwError::Refused {
                device: Device::Printer,
                detail: format!("lp: {}", output.stderr.trim()),
            })
        }
    }

    fn status(&self) -> PrinterStatus {
        let args = ["-p", self.queue.as_str()];
        match run_program(Device::Printer, "lpstat", &args, b"", LPSTAT_TIMEOUT) {
            // CUPS writes "printer X disabled since …" for a stopped queue.
            Ok(output) if output.success && output.stdout.contains("disabled") => {
                PrinterStatus::Offline {
                    detail: output.stdout.trim().to_string(),
                }
            }
            Ok(output) if output.success => PrinterStatus::Ready,
            Ok(output) => PrinterStatus::Offline {
                detail: output.stderr.trim().to_string(),
            },
            Err(error) => PrinterStatus::Offline {
                detail: error.to_string(),
            },
        }
    }
}

/// The names of the CUPS queues, from `lpstat -e` (one name per line).
#[cfg(not(windows))]
pub(crate) fn installed_queues() -> Result<Vec<String>, HwError> {
    let output = run_program(Device::Printer, "lpstat", &["-e"], b"", LPSTAT_TIMEOUT)?;
    Ok(queue_names(&output.stdout))
}

/// One queue name per line; blank lines are skipped.
#[cfg(not(windows))]
fn queue_names(lpstat_output: &str) -> Vec<String> {
    lpstat_output
        .lines()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(all(test, not(windows)))]
mod tests {
    use super::*;

    #[test]
    fn hw_02_queue_names_are_read_one_per_line() {
        let names = queue_names("EPSON_TM_T20\n\nXprinter_raw\n");
        assert_eq!(names, vec!["EPSON_TM_T20", "Xprinter_raw"]);
    }
}
