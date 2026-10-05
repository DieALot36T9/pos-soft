//! A serial or USB-serial printer, through the `escpos` crate's serial driver.

use std::time::Duration;

use escpos::driver::{Driver, SerialPortDriver};

use super::{PrinterStatus, RawPrinter, escpos_error};
use crate::HwError;

/// The longest wait for one write to the port. A slow port (9600 baud) takes about
/// one second per kilobyte, and the data goes out in several writes.
const SERIAL_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) struct SerialPrinter {
    port: String,
    baud: u32,
}

impl SerialPrinter {
    pub(crate) fn new(port: String, baud: u32) -> SerialPrinter {
        SerialPrinter { port, baud }
    }
}

impl RawPrinter for SerialPrinter {
    fn send(&self, bytes: &[u8]) -> Result<(), HwError> {
        let driver = SerialPortDriver::open(&self.port, self.baud, Some(SERIAL_TIMEOUT))
            .map_err(|error| escpos_error(error, true))?;
        driver
            .write(bytes)
            .and_then(|()| driver.flush())
            .map_err(|error| escpos_error(error, false))
    }

    fn status(&self) -> PrinterStatus {
        // Opening the port to check it could disturb a print in progress, so we only
        // check that the port exists.
        let exists = crate::list_serial_ports()
            .iter()
            .any(|port| port.name == self.port);
        if exists {
            PrinterStatus::Unknown
        } else {
            PrinterStatus::Offline {
                detail: format!("serial port {} was not found", self.port),
            }
        }
    }
}
