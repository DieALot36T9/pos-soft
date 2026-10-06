//! The things every printout and device call needs, made once at start-up:
//! the bundled fonts, the Preview printer's inbox, the fake scale, the print queue
//! and the printouts that failed and wait for "Retry" (rule HW-08).

use std::path::PathBuf;
use std::sync::Mutex;

use hardware::{
    DataBits, FakeScale, Parity, PreviewInbox, PrinterConfig, RawPrinter, Scale, SerialSettings,
    StopBits, builtin_scale_profiles, printer_from, scale_from,
};
use pos_core::AppError;
use printing::{Fonts, load_fonts};
use settings::{HardwareSettings, PrinterConnection};

use crate::state::lock;
use crate::workflows::receipts::Printable;

/// Shared by every command. Tauri keeps one in its state.
pub struct Devices {
    /// The fonts receipts are drawn with. Loading them takes a moment, so it happens once.
    pub fonts: Fonts,
    /// Where the Preview printer leaves each printout until the screen takes it.
    pub preview: PreviewInbox,
    /// The scale behind the dev-only fake scale panel.
    pub fake_scale: FakeScale,
    /// The folder where preview printouts are saved as PNG files. None: not saved.
    pub prints_folder: Option<PathBuf>,
    /// Held while bytes go to the receipt printer, so two printouts never mix.
    print_queue: Mutex<()>,
    /// Printouts that failed and may be sent again ("Retry") or printed on the
    /// system printer (PAY-09). Only in memory.
    unprinted: Mutex<Vec<Printable>>,
}

impl Devices {
    /// Loads the fonts and starts with an empty preview inbox and an empty fake scale.
    pub fn new(prints_folder: Option<PathBuf>) -> Devices {
        Devices {
            fonts: load_fonts(),
            preview: PreviewInbox::new(),
            fake_scale: FakeScale::new(),
            prints_folder,
            print_queue: Mutex::new(()),
            unprinted: Mutex::new(Vec::new()),
        }
    }

    /// The receipt printer from the settings. Until one is chosen, it is the Preview
    /// printer, so no device is needed (rule HW-10).
    pub fn receipt_printer(&self, hardware: &HardwareSettings) -> Box<dyn RawPrinter> {
        let config = match &hardware.receipt_printer {
            None => PrinterConfig::Preview,
            Some(connection) => printer_config(connection),
        };
        printer_from(&config, &self.preview)
    }

    /// Sends one printout's bytes. Waits for any printout already being sent.
    pub fn send(&self, printer: &dyn RawPrinter, bytes: &[u8]) -> Result<(), AppError> {
        let _turn = lock(&self.print_queue)?;
        printer.send(bytes)?;
        Ok(())
    }

    /// The scale checkout reads: the one in the settings, or else the fake one when
    /// the dev tools are built in. Without one, the weight dialog asks for the weight
    /// to be typed (CHK-06).
    pub fn scale(&self, hardware: &HardwareSettings) -> Option<Box<dyn Scale>> {
        if let Some(saved) = &hardware.scale {
            let profile = builtin_scale_profiles()
                .into_iter()
                .find(|(id, _)| *id == saved.profile)
                .map(|(_, profile)| profile)?;
            let port = SerialSettings {
                port: saved.port.clone(),
                baud: saved.baud,
                data_bits: DataBits::Eight,
                parity: Parity::None,
                stop_bits: StopBits::One,
            };
            return Some(scale_from(&port, &profile));
        }
        if crate::dev_tools::ON {
            Some(Box::new(self.fake_scale.clone()))
        } else {
            None
        }
    }

    /// Remembers a printout that failed, so "Retry" may send it again.
    pub(crate) fn mark_unprinted(&self, printable: &Printable) -> Result<(), AppError> {
        let mut waiting = lock(&self.unprinted)?;
        if !waiting.contains(printable) {
            waiting.push(printable.clone());
        }
        Ok(())
    }

    /// Forgets a printout once it printed, or the cashier dismissed the banner.
    pub(crate) fn mark_printed(&self, printable: &Printable) -> Result<(), AppError> {
        lock(&self.unprinted)?.retain(|each| each != printable);
        Ok(())
    }

    /// True while a printout failed and has not printed since.
    pub(crate) fn is_unprinted(&self, printable: &Printable) -> Result<bool, AppError> {
        Ok(lock(&self.unprinted)?.contains(printable))
    }
}

/// The settings module and the hardware module each describe a printer connection.
fn printer_config(connection: &PrinterConnection) -> PrinterConfig {
    match connection {
        PrinterConnection::WindowsSpooler { printer_name } => PrinterConfig::WindowsSpooler {
            printer_name: printer_name.clone(),
        },
        PrinterConnection::Tcp { host, port } => PrinterConfig::Tcp {
            host: host.clone(),
            port: *port,
        },
        PrinterConnection::Serial { port, baud } => PrinterConfig::Serial {
            port: port.clone(),
            baud: *baud,
        },
        PrinterConnection::Cups { queue } => PrinterConfig::Cups {
            queue: queue.clone(),
        },
        PrinterConnection::Preview => PrinterConfig::Preview,
        PrinterConnection::File { path } => PrinterConfig::File {
            path: PathBuf::from(path),
        },
    }
}
