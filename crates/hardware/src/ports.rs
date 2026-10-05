//! Finding the serial ports on this computer, for the printer and scale settings.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One serial port the user can choose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SerialPortInfo {
    /// What to put in the settings. Example: "COM3", "/dev/cu.usbserial-1410".
    pub name: String,
    pub kind: PortKind,
    /// The device's own name, if it gives one. Example: "USB-Serial Controller".
    pub product: Option<String>,
}

/// How the port is connected, so the screen can show a hint next to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum PortKind {
    Usb,
    Bluetooth,
    Pci,
    Other,
}

/// The serial and USB-serial ports on this computer, sorted by name.
/// Returns an empty list if the computer can't tell.
pub fn list_serial_ports() -> Vec<SerialPortInfo> {
    let found = serialport::available_ports().unwrap_or_default();
    let mut ports: Vec<SerialPortInfo> = found
        .into_iter()
        .filter(|port| is_usable_port(&port.port_name, cfg!(target_os = "macos")))
        .map(|port| {
            let (kind, product) = match port.port_type {
                serialport::SerialPortType::UsbPort(usb) => (PortKind::Usb, usb.product),
                serialport::SerialPortType::BluetoothPort => (PortKind::Bluetooth, None),
                serialport::SerialPortType::PciPort => (PortKind::Pci, None),
                serialport::SerialPortType::Unknown => (PortKind::Other, None),
            };
            SerialPortInfo {
                name: port.port_name,
                kind,
                product,
            }
        })
        .collect();
    ports.sort_by(|first, second| first.name.cmp(&second.name));
    ports
}

/// On macOS every port shows up twice: `/dev/tty.*` waits for a modem signal that
/// printers and scales never send, so only `/dev/cu.*` is offered.
fn is_usable_port(name: &str, on_mac: bool) -> bool {
    !(on_mac && name.starts_with("/dev/tty."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hw_05_on_macos_only_cu_ports_are_offered() {
        assert!(is_usable_port("/dev/cu.usbserial-1410", true));
        assert!(!is_usable_port("/dev/tty.usbserial-1410", true));
        assert!(is_usable_port("COM3", false));
        assert!(is_usable_port("/dev/ttyUSB0", false));
    }
}
