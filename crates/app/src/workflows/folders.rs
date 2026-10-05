//! Checking that backup folders can be written to (rule SET-11).

use std::path::Path;

use pos_core::AppError;
use settings::BackupSettings;

use crate::workflows::setup::field_error;

/// Refuses a backup folder the app can't create or write to. An empty main
/// folder means the app's own `backups` folder, which always works.
pub fn check_backup_folders(backup: &BackupSettings) -> Result<(), AppError> {
    if !backup.primary_dir.trim().is_empty() && !can_write(Path::new(backup.primary_dir.trim())) {
        return Err(field_error("primary_dir", "folder_not_writable"));
    }
    if let Some(second) = &backup.secondary_dir
        && !second.trim().is_empty()
        && !can_write(Path::new(second.trim()))
    {
        return Err(field_error("secondary_dir", "folder_not_writable"));
    }
    Ok(())
}

/// Makes the folder if needed and writes, then removes, a small test file.
fn can_write(folder: &Path) -> bool {
    let probe = folder.join(".supershop-write-test");
    std::fs::create_dir_all(folder).is_ok()
        && std::fs::write(&probe, b"ok").is_ok()
        && std::fs::remove_file(&probe).is_ok()
}
