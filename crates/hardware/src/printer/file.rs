//! For development: a "printer" that adds every job to the end of a file.

use std::fs::OpenOptions;
use std::path::PathBuf;
use std::time::Duration;

use escpos::driver::{Driver, FileDriver};

use super::{PrinterStatus, RawPrinter, escpos_error};
use crate::wait::run_with_timeout;
use crate::{Device, HwError};

/// A file on a network drive can hang, so writing has a time limit too.
const FILE_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) struct FilePrinter {
    path: PathBuf,
}

impl FilePrinter {
    pub(crate) fn new(path: PathBuf) -> FilePrinter {
        FilePrinter { path }
    }
}

impl RawPrinter for FilePrinter {
    fn send(&self, bytes: &[u8]) -> Result<(), HwError> {
        let path = self.path.clone();
        let bytes = bytes.to_vec();
        run_with_timeout(Device::Printer, FILE_TIMEOUT, move || {
            let mut options = OpenOptions::new();
            options.create(true).append(true);
            let driver = FileDriver::open_with_options(&path, &options)
                .map_err(|error| escpos_error(error, true))?;
            driver
                .write(&bytes)
                .and_then(|()| driver.flush())
                .map_err(|error| escpos_error(error, false))
        })
    }

    fn status(&self) -> PrinterStatus {
        let folder = self
            .path
            .parent()
            .filter(|folder| !folder.as_os_str().is_empty());
        match folder {
            Some(folder) if !folder.is_dir() => PrinterStatus::Offline {
                detail: format!("folder {} doesn't exist", folder.display()),
            },
            _ => PrinterStatus::Ready,
        }
    }
}
