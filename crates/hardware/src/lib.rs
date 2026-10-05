//! Printer connections, reading the scale and finding devices.
//! This crate sends bytes and reads weights. It knows nothing about sales or receipts.
//!
//! Page: `docs/05 Modules/Module - hardware.md`.

mod error;
mod fiscal;
mod ports;
mod printer;
mod scale;
mod wait;

pub use error::{Device, HwError};
pub use fiscal::FiscalDevice;
pub use ports::{PortKind, SerialPortInfo, list_serial_ports};
pub use printer::{
    MockPrinter, PreviewInbox, PreviewJob, PrinterConfig, PrinterStatus, RawPrinter, list_printers,
    printer_from, read_escpos_job,
};
pub use scale::{
    DataBits, FakeScale, FakeScaleState, MockScale, Parity, Reading, Scale, ScaleMode,
    ScaleProfile, SerialSettings, StopBits, WeightUnit, builtin_scale_profiles, check_profile,
    scale_from,
};
