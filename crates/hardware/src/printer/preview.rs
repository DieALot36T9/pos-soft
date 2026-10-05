//! The Preview printer, for working without a printer (rule HW-10).
//! It acts like a receipt printer: it reads the ESC/POS bytes it gets and draws the
//! pictures in them. `app` shows those pictures in a window and saves them as PNG files.

use std::sync::{Arc, Mutex};

use pos_core::Bitmap1;

use super::{PrinterStatus, RawPrinter};
use crate::HwError;
use crate::wait::lock;

const ESC: u8 = 0x1B;
const GS: u8 = 0x1D;
/// `GS v 0`: print a raster picture. The `escpos` crate and the `printing` module send pictures this way.
const RASTER_PICTURE: [u8; 3] = [GS, b'v', b'0'];
/// `ESC p`: send a pulse to the cash drawer.
const DRAWER_PULSE: [u8; 2] = [ESC, b'p'];
/// `GS v 0 m xL xH yL yH`: the command, then the size in 5 bytes.
const RASTER_HEADER_LENGTH: usize = 8;
/// `ESC p m`: the pin follows the command. Pulse times may follow too; they are skipped as plain bytes.
const DRAWER_PULSE_LENGTH: usize = 3;

/// What the Preview printer saw in one job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewJob {
    /// The pictures the printer would print, top to bottom. Picture bands of the same
    /// width that follow each other are joined into one picture.
    pub pictures: Vec<Bitmap1>,
    /// True if the job asked to open the cash drawer. The preview shows "Drawer opened".
    pub drawer_opened: bool,
    /// The bytes exactly as they were sent.
    pub bytes: Vec<u8>,
}

/// The place where the Preview printer leaves its jobs. `app` keeps one, gives it to
/// `printer_from`, and takes the jobs out to show them. Copies share the same jobs.
#[derive(Debug, Clone, Default)]
pub struct PreviewInbox {
    jobs: Arc<Mutex<Vec<PreviewJob>>>,
}

impl PreviewInbox {
    /// An empty inbox.
    pub fn new() -> PreviewInbox {
        PreviewInbox::default()
    }

    /// Takes out every job printed since the last call, oldest first.
    pub fn take_jobs(&self) -> Vec<PreviewJob> {
        std::mem::take(&mut *lock(&self.jobs))
    }
}

pub(crate) struct PreviewPrinter {
    inbox: PreviewInbox,
}

impl PreviewPrinter {
    pub(crate) fn new(inbox: PreviewInbox) -> PreviewPrinter {
        PreviewPrinter { inbox }
    }
}

impl RawPrinter for PreviewPrinter {
    fn send(&self, bytes: &[u8]) -> Result<(), HwError> {
        let job = read_escpos_job(bytes);
        lock(&self.inbox.jobs).push(job);
        Ok(())
    }

    fn status(&self) -> PrinterStatus {
        PrinterStatus::Ready
    }
}

/// Reads ESC/POS bytes the way a printer would, and keeps the pictures and the drawer pulse.
/// Other commands and text are skipped: receipts are always sent as pictures (ADR-012).
pub fn read_escpos_job(bytes: &[u8]) -> PreviewJob {
    let mut job = PreviewJob {
        pictures: Vec::new(),
        drawer_opened: false,
        bytes: bytes.to_vec(),
    };
    let mut position = 0;
    while position < bytes.len() {
        let rest = &bytes[position..];
        if rest.starts_with(&RASTER_PICTURE) {
            match read_raster_band(rest) {
                Some((band, length)) => {
                    add_band(&mut job.pictures, band);
                    position += length;
                }
                // The picture was cut off. A real printer would stop here too.
                None => break,
            }
        } else if rest.starts_with(&DRAWER_PULSE) {
            job.drawer_opened = true;
            position += DRAWER_PULSE_LENGTH;
        } else {
            position += 1;
        }
    }
    job
}

/// Reads one `GS v 0` picture band. Returns the picture and how many bytes it used,
/// or `None` if the bytes end before the picture does.
fn read_raster_band(bytes: &[u8]) -> Option<(Bitmap1, usize)> {
    let header = bytes.get(..RASTER_HEADER_LENGTH)?;
    // header[3] is the size mode (normal, double width…). The preview always draws at normal size.
    let bytes_per_row = usize::from(header[4]) + usize::from(header[5]) * 256;
    let height = usize::from(header[6]) + usize::from(header[7]) * 256;
    let end = RASTER_HEADER_LENGTH + bytes_per_row * height;
    let rows = bytes.get(RASTER_HEADER_LENGTH..end)?;
    let band = Bitmap1 {
        width: u32::try_from(bytes_per_row * 8).ok()?,
        height: u32::try_from(height).ok()?,
        bits: rows.to_vec(),
    };
    Some((band, end))
}

/// Adds a band under the last picture if it has the same width, or starts a new picture.
fn add_band(pictures: &mut Vec<Bitmap1>, band: Bitmap1) {
    match pictures.last_mut() {
        Some(last) if last.width == band.width => {
            last.height += band.height;
            last.bits.extend_from_slice(&band.bits);
        }
        _ => pictures.push(band),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `GS v 0` band: `bytes_per_row` × `rows.len()`, every row filled with its byte.
    fn raster_band(bytes_per_row: u8, rows: &[u8]) -> Vec<u8> {
        let height = u8::try_from(rows.len()).unwrap();
        let mut bytes = vec![GS, b'v', b'0', 0, bytes_per_row, 0, height, 0];
        for row in rows {
            bytes.extend(std::iter::repeat_n(*row, usize::from(bytes_per_row)));
        }
        bytes
    }

    #[test]
    fn hw_10_preview_draws_the_picture_it_was_sent() {
        let mut bytes = vec![ESC, b'@'];
        bytes.extend(raster_band(2, &[0xFF, 0x00, 0x81]));
        bytes.extend([GS, b'V', 0]);

        let job = read_escpos_job(&bytes);

        assert_eq!(
            job.pictures,
            vec![Bitmap1 {
                width: 16,
                height: 3,
                bits: vec![0xFF, 0xFF, 0x00, 0x00, 0x81, 0x81],
            }]
        );
        assert!(!job.drawer_opened);
        assert_eq!(job.bytes, bytes);
    }

    #[test]
    fn hw_10_bands_of_one_receipt_join_into_one_picture() {
        let mut bytes = raster_band(1, &[0x01, 0x02]);
        bytes.extend(raster_band(1, &[0x03]));

        let job = read_escpos_job(&bytes);

        assert_eq!(job.pictures.len(), 1);
        assert_eq!(job.pictures[0].height, 3);
        assert_eq!(job.pictures[0].bits, vec![0x01, 0x02, 0x03]);
    }

    #[test]
    fn a_band_of_another_width_starts_a_new_picture() {
        let mut bytes = raster_band(1, &[0x01]);
        bytes.extend(raster_band(2, &[0x02]));

        let job = read_escpos_job(&bytes);

        assert_eq!(job.pictures.len(), 2);
        assert_eq!(job.pictures[1].width, 16);
    }

    #[test]
    fn hw_10_drawer_pulse_is_noticed() {
        // ESC p, pin 2, pulse on 25 × 2 ms, off 250 × 2 ms.
        let job = read_escpos_job(&[ESC, b'p', 0, 25, 250]);
        assert!(job.drawer_opened);
        assert!(job.pictures.is_empty());
    }

    #[test]
    fn picture_bytes_that_look_like_a_drawer_pulse_are_not_one() {
        let job = read_escpos_job(&raster_band(2, &[ESC, b'p']));
        assert!(!job.drawer_opened);
        assert_eq!(job.pictures.len(), 1);
    }

    #[test]
    fn a_cut_off_picture_is_dropped() {
        let mut bytes = raster_band(2, &[0xFF, 0xFF]);
        bytes.truncate(bytes.len() - 1);
        let job = read_escpos_job(&bytes);
        assert!(job.pictures.is_empty());
    }

    #[test]
    fn hw_10_inbox_hands_out_each_job_once() {
        let inbox = PreviewInbox::new();
        let printer = PreviewPrinter::new(inbox.clone());

        printer.send(&[ESC, b'p', 1]).unwrap();
        printer.send(&raster_band(1, &[0xAA])).unwrap();

        let jobs = inbox.take_jobs();
        assert_eq!(jobs.len(), 2);
        assert!(jobs[0].drawer_opened);
        assert_eq!(jobs[1].pictures.len(), 1);
        assert!(inbox.take_jobs().is_empty());
        assert_eq!(printer.status(), PrinterStatus::Ready);
    }
}
