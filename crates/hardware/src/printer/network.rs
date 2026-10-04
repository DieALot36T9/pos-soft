//! A network printer (usually port 9100), through the `escpos` crate's network driver.

use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use escpos::driver::{Driver, NetworkDriver};

use super::{PrinterStatus, RawPrinter, escpos_error};
use crate::{Device, HwError};

/// How long to wait for the printer to accept the connection, and for each write.
pub(crate) const NETWORK_TIMEOUT: Duration = Duration::from_secs(3);

pub(crate) struct NetworkPrinter {
    host: String,
    port: u16,
}

impl NetworkPrinter {
    pub(crate) fn new(host: String, port: u16) -> NetworkPrinter {
        NetworkPrinter { host, port }
    }

    /// Finds the printer's address. An IP address is used as it is; a name is looked up.
    fn address(&self) -> Result<SocketAddr, HwError> {
        let not_found = |detail: String| HwError::NotConnected {
            device: Device::Printer,
            detail,
        };
        let mut addresses = (self.host.as_str(), self.port)
            .to_socket_addrs()
            .map_err(|error| not_found(format!("{}: {error}", self.host)))?;
        addresses
            .next()
            .ok_or_else(|| not_found(format!("{} has no address", self.host)))
    }
}

impl RawPrinter for NetworkPrinter {
    fn send(&self, bytes: &[u8]) -> Result<(), HwError> {
        let address = self.address()?;
        // The escpos driver puts the time limit on connecting and on every write.
        let driver = NetworkDriver::open(
            &address.ip().to_string(),
            address.port(),
            Some(NETWORK_TIMEOUT),
        )
        .map_err(|error| escpos_error(error, true))?;
        driver
            .write(bytes)
            .and_then(|()| driver.flush())
            .map_err(|error| escpos_error(error, false))
    }

    fn status(&self) -> PrinterStatus {
        let reached = self.address().and_then(|address| {
            TcpStream::connect_timeout(&address, NETWORK_TIMEOUT).map_err(|error| {
                HwError::NotConnected {
                    device: Device::Printer,
                    detail: error.to_string(),
                }
            })
        });
        match reached {
            Ok(_) => PrinterStatus::Ready,
            Err(error) => PrinterStatus::Offline {
                detail: error.to_string(),
            },
        }
    }
}
