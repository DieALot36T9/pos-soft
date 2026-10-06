//! The things every printout and device call needs, made once at start-up:
//! the bundled fonts, the Preview printer's inbox and the fake scale.

use std::path::PathBuf;

use hardware::{FakeScale, PreviewInbox, PrinterConfig, RawPrinter, Scale, printer_from};
use printing::{Fonts, load_fonts};

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
}

impl Devices {
    /// Loads the fonts and starts with an empty preview inbox and an empty fake scale.
    pub fn new(prints_folder: Option<PathBuf>) -> Devices {
        Devices {
            fonts: load_fonts(),
            preview: PreviewInbox::new(),
            fake_scale: FakeScale::new(),
            prints_folder,
        }
    }

    /// The receipt printer. Until printer settings are saved (C-M2, C-M7), it is
    /// always the Preview printer, so no device is needed (rule HW-10).
    pub fn receipt_printer(&self) -> Box<dyn RawPrinter> {
        printer_from(&PrinterConfig::Preview, &self.preview)
    }

    /// The scale checkout reads. Until scale settings are used (C-M7), the only scale
    /// is the fake one, and only when the dev tools are built in. Without one, the
    /// weight dialog asks for the weight to be typed (CHK-06).
    pub fn scale(&self) -> Option<&dyn Scale> {
        if crate::dev_tools::ON {
            Some(&self.fake_scale)
        } else {
            None
        }
    }
}
