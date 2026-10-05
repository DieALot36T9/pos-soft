//! A pretend printer for tests. It behaves like a real one: it can be slow and it can fail.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use super::{PrinterStatus, RawPrinter};
use crate::HwError;
use crate::wait::lock;

/// A pretend printer that keeps every job it was sent.
pub struct MockPrinter {
    /// Every job that "printed", oldest first.
    pub sent: Mutex<Vec<Vec<u8>>>,
    failures: Mutex<VecDeque<HwError>>,
    delay: Duration,
    status: Mutex<PrinterStatus>,
}

impl MockPrinter {
    /// A pretend printer that prints at once and never fails.
    pub fn new() -> MockPrinter {
        MockPrinter {
            sent: Mutex::new(Vec::new()),
            failures: Mutex::new(VecDeque::new()),
            delay: Duration::ZERO,
            status: Mutex::new(PrinterStatus::Ready),
        }
    }

    /// The same printer, but every job takes this long, like a real slow printer.
    pub fn with_delay(mut self, delay: Duration) -> MockPrinter {
        self.delay = delay;
        self
    }

    /// The next job fails with this error and prints nothing. Call it again to queue more failures.
    pub fn fail_next(&self, error: HwError) {
        lock(&self.failures).push_back(error);
    }

    /// Sets what `status` answers. Example: `Offline` while testing a "printer off" banner.
    pub fn set_status(&self, status: PrinterStatus) {
        *lock(&self.status) = status;
    }

    /// A copy of every job printed so far, oldest first.
    pub fn sent_jobs(&self) -> Vec<Vec<u8>> {
        lock(&self.sent).clone()
    }
}

impl Default for MockPrinter {
    fn default() -> MockPrinter {
        MockPrinter::new()
    }
}

impl RawPrinter for MockPrinter {
    fn send(&self, bytes: &[u8]) -> Result<(), HwError> {
        thread::sleep(self.delay);
        if let Some(error) = lock(&self.failures).pop_front() {
            return Err(error);
        }
        lock(&self.sent).push(bytes.to_vec());
        Ok(())
    }

    fn status(&self) -> PrinterStatus {
        lock(&self.status).clone()
    }
}
