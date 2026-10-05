//! Where the app keeps its files, all inside the app's data folder.

use std::path::{Path, PathBuf};

use pos_core::AppError;

/// Every file and folder the app uses. Tests point it at a temporary folder.
#[derive(Debug, Clone)]
pub struct AppPaths {
    /// The app's data folder. Example on Windows: `%LOCALAPPDATA%\com.supershop.pos`.
    pub data_dir: PathBuf,
}

impl AppPaths {
    /// Paths inside this data folder.
    pub fn new(data_dir: &Path) -> AppPaths {
        AppPaths {
            data_dir: data_dir.to_path_buf(),
        }
    }

    /// The shop's database.
    pub fn database(&self) -> PathBuf {
        self.data_dir.join("pos.sqlite")
    }

    /// The default backup folder (rule SET-11).
    pub fn backups(&self) -> PathBuf {
        self.data_dir.join("backups")
    }

    /// Daily log files (rule BAK-07).
    pub fn logs(&self) -> PathBuf {
        self.data_dir.join("logs")
    }

    /// Preview printer pictures (rule HW-10).
    pub fn prints(&self) -> PathBuf {
        self.data_dir.join("dev-prints")
    }

    /// Written before a restart that restores a backup; read and removed at the next start.
    pub fn restore_marker(&self) -> PathBuf {
        self.data_dir.join("restore-pending.json")
    }

    /// Makes the data, backup and log folders if they are missing.
    pub fn create_folders(&self) -> Result<(), AppError> {
        for folder in [self.data_dir.clone(), self.backups(), self.logs()] {
            std::fs::create_dir_all(&folder).map_err(|error| {
                AppError::internal(format!("could not make the folder {folder:?}: {error}"))
            })?;
        }
        Ok(())
    }
}
