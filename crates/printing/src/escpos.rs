//! ESC/POS bytes for receipt printers (Epson's command set, rules HW-02 and HW-04).
//! The `escpos` crate makes the start and cut commands. Two commands are written here:
//! - the picture, sent in strips with the raster command `GS v 0`: the crate only
//!   prints pictures from image files, which it decodes, shrinks to 512 dots and
//!   converts again, while our picture is already in the printer's own format;
//! - the drawer pulse `ESC p m t1 t2`: the crate (0.20) leaves out `t1` and `t2`,
//!   so the printer would take the next two bytes it receives as the timings.

use ::escpos::driver::Driver;
use ::escpos::errors::{PrinterError, Result as EscPosResult};
use ::escpos::printer::Printer;
use ::escpos::utils::Protocol;
use std::cell::RefCell;
use std::rc::Rc;

use pos_core::Bitmap1;

use crate::PrintError;
use crate::pictures::{bytes_per_row, check_picture};

/// `GS v 0` with mode 0 (normal size): print a raster picture.
const RASTER_PICTURE: [u8; 4] = [0x1D, b'v', b'0', 0];
/// `ESC p`: send a pulse to a drawer pin.
const DRAWER_PULSE: [u8; 2] = [0x1B, b'p'];
/// Pulse on for 25 × 2 ms = 50 ms, then off for 250 × 2 ms = 500 ms: the usual
/// values in Epson's examples, long enough for common drawers.
const PULSE_ON: u8 = 25;
const PULSE_OFF: u8 = 250;

/// Which drawer pin the printer pulses to open the cash drawer (rule HW-04).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawerPin {
    Pin2,
    Pin5,
}

/// Options for one printout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EscPosOpts {
    /// Cut the paper after the picture.
    pub cut: bool,
    /// Open the cash drawer after the cut.
    pub drawer: Option<DrawerPin>,
    /// Rows per strip. Some printers have a small memory and need short strips.
    /// 0 counts as 1.
    pub band_height: u32,
}

impl Default for EscPosOpts {
    /// Cut, no drawer, strips of 256 rows.
    fn default() -> EscPosOpts {
        EscPosOpts {
            cut: true,
            drawer: None,
            band_height: 256,
        }
    }
}

/// Turns a picture into ESC/POS bytes: start the printer fresh, send the picture
/// in strips, then cut and open the drawer if asked.
pub fn encode(b: &Bitmap1, opts: &EscPosOpts) -> Result<Vec<u8>, PrintError> {
    check_picture(b)?;
    let row_bytes = bytes_per_row(b);
    let width_bytes = u16::try_from(row_bytes).map_err(|_| PrintError::TooBig {
        width: b.width,
        height: b.height,
    })?;
    let band_height = opts.band_height.clamp(1, u32::from(u16::MAX));

    let mut printer = memory_printer();
    printer.init().map_err(escpos_error)?;
    let mut top = 0;
    while top < b.height {
        let rows = band_height.min(b.height - top);
        let mut command = RASTER_PICTURE.to_vec();
        command.extend_from_slice(&width_bytes.to_le_bytes());
        // `rows` is at most `band_height`, which fits in two bytes.
        command.extend_from_slice(&(rows as u16).to_le_bytes());
        let start = top as usize * row_bytes;
        let end = (top + rows) as usize * row_bytes;
        command.extend_from_slice(&b.bits[start..end]);
        printer.custom(&command).map_err(escpos_error)?;
        top += rows;
    }
    if opts.cut {
        printer.cut().map_err(escpos_error)?;
    }
    if let Some(pin) = opts.drawer {
        printer.custom(&pulse_command(pin)).map_err(escpos_error)?;
    }
    finish(printer)
}

/// The bytes that open the cash drawer on its own, for no-sale and cash in/out (rule HW-04).
pub fn drawer_pulse(pin: DrawerPin) -> Result<Vec<u8>, PrintError> {
    let mut printer = memory_printer();
    printer.custom(&pulse_command(pin)).map_err(escpos_error)?;
    finish(printer)
}

/// `ESC p m t1 t2`. `m` is 0 for pin 2 and 1 for pin 5.
fn pulse_command(pin: DrawerPin) -> [u8; 5] {
    let pin_number = match pin {
        DrawerPin::Pin2 => 0,
        DrawerPin::Pin5 => 1,
    };
    [
        DRAWER_PULSE[0],
        DRAWER_PULSE[1],
        pin_number,
        PULSE_ON,
        PULSE_OFF,
    ]
}

/// A printer that writes into memory. Sending the bytes is the `hardware` crate's job.
fn memory_printer() -> Printer<MemoryDriver> {
    Printer::new(MemoryDriver::default(), Protocol::default(), None)
}

/// Collects what the `escpos` printer has written.
fn finish(mut printer: Printer<MemoryDriver>) -> Result<Vec<u8>, PrintError> {
    printer.print().map_err(escpos_error)?;
    let written = printer.driver().bytes.take();
    Ok(written)
}

fn escpos_error(error: PrinterError) -> PrintError {
    PrintError::EscPos(error.to_string())
}

/// An `escpos` driver that keeps the bytes in memory instead of sending them.
#[derive(Default, Clone)]
struct MemoryDriver {
    bytes: Rc<RefCell<Vec<u8>>>,
}

impl Driver for MemoryDriver {
    fn name(&self) -> String {
        "memory".to_string()
    }

    fn write(&self, data: &[u8]) -> EscPosResult<()> {
        self.bytes.borrow_mut().extend_from_slice(data);
        Ok(())
    }

    fn read(&self, _buf: &mut [u8]) -> EscPosResult<usize> {
        Ok(0)
    }

    fn flush(&self) -> EscPosResult<()> {
        Ok(())
    }
}
