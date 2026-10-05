//! The Preview printer's printouts for the screen (rule HW-10): each picture as a
//! PNG, exactly as it would print, also saved as a PNG file.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use pos_core::AppError;
use printing::to_png;
use serde::Serialize;
use ts_rs::TS;

use crate::devices::Devices;

/// One job the Preview printer received, ready to show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct PreviewPrintout {
    /// Each picture as a `data:image/png;base64,…` address, top to bottom.
    pub pictures: Vec<String>,
    /// True if the job opened the cash drawer. The screen shows "Drawer opened".
    pub drawer_opened: bool,
    /// Where each picture was saved, if saving is on.
    pub saved_files: Vec<String>,
}

/// Takes every printout since the last call, oldest first, and saves its pictures.
pub fn take_printouts(devices: &Devices) -> Result<Vec<PreviewPrintout>, AppError> {
    let mut printouts = Vec::new();
    for (job_number, job) in devices.preview.take_jobs().into_iter().enumerate() {
        let mut pictures = Vec::new();
        let mut saved_files = Vec::new();
        for (picture_number, picture) in job.pictures.iter().enumerate() {
            let png = to_png(picture)?;
            if let Some(folder) = &devices.prints_folder {
                let name = format!("{}-{job_number}-{picture_number}.png", millis_now());
                saved_files.push(save(folder, &name, &png)?);
            }
            pictures.push(format!("data:image/png;base64,{}", STANDARD.encode(&png)));
        }
        printouts.push(PreviewPrintout {
            pictures,
            drawer_opened: job.drawer_opened,
            saved_files,
        });
    }
    Ok(printouts)
}

/// Saves one PNG and gives back its full path.
fn save(folder: &Path, name: &str, png: &[u8]) -> Result<String, AppError> {
    std::fs::create_dir_all(folder).map_err(|error| {
        AppError::internal(format!(
            "could not make the prints folder {folder:?}: {error}"
        ))
    })?;
    let path = folder.join(name);
    std::fs::write(&path, png)
        .map_err(|error| AppError::internal(format!("could not save {path:?}: {error}")))?;
    Ok(path.to_string_lossy().into_owned())
}

/// Milliseconds since 1970, so saved files sort by the time they were printed.
fn millis_now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis())
        .unwrap_or_default()
}
