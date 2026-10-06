//! Backups (rules BAK-01 to BAK-05): make one, copy it to the second folder, prune
//! old ones, list them for the restore screen, and ask for a restore.

use std::path::{Path, PathBuf};

use pos_core::{AppError, Id};
use serde::Serialize;
use settings::BackupSettings;
use ts_rs::TS;

use crate::migrations;
use crate::startup::{RestoreRequest, ask_for_restore};
use crate::state::{AppState, DbHealth, lock};

/// Besides the newest `keep_last`, the last backup of each day is kept this many days.
const KEEP_DAILY_DAYS: u32 = 30;

/// A backup that was made, and any problem with the second copy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct BackupDone {
    pub file: String,
    /// An error code for the warning banner, e.g. `backup_second_folder_failed`.
    pub warning: Option<String>,
}

/// One backup in the restore list (BAK-05).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct BackupRow {
    pub file: String,
    pub file_name: String,
    /// When it was made, local time. Example: "2026-10-05 14:30:00".
    pub taken_at: String,
    /// Why it was made. Example: "auto", "close", "manual", "pre-restore".
    pub reason: String,
    #[ts(type = "number")]
    pub size_bytes: u64,
    /// False when the file fails its check or comes from a newer app. It can't be restored.
    pub usable: bool,
    pub sale_count: u32,
    pub last_invoice_no: Option<String>,
}

/// The backup settings, with the default folder filled in when none is set.
pub fn backup_settings(state: &AppState) -> Result<BackupSettings, AppError> {
    let mut saved: BackupSettings = state.db.read(|conn| Ok(settings::get(conn)?))?;
    if saved.primary_dir.trim().is_empty() {
        saved.primary_dir = state.paths.backups().to_string_lossy().into_owned();
    }
    Ok(saved)
}

/// Makes a backup now (VACUUM INTO + check), copies it to the second folder and
/// deletes old ones. A problem with the second folder is a warning, never an error (BAK-02).
pub fn make_backup(state: &AppState, reason: &str) -> Result<BackupDone, AppError> {
    let options = backup_settings(state)?;
    let primary = PathBuf::from(&options.primary_dir);
    create_folder(&primary)?;
    let marker = db::change_marker(&state.db);
    let file = free_file_name(&primary, state, reason);
    if let Err(error) = db::backup_to(&state.db, &file) {
        *lock(&state.backup_warning)? = Some("backup_check_failed".into());
        return Err(error.into());
    }
    *lock(&state.backed_up_at_marker)? = marker;
    let keep = usize::try_from(options.keep_last).unwrap_or(usize::MAX);
    prune(&primary, keep, state);
    let warning = match &options.secondary_dir {
        Some(folder) if !folder.trim().is_empty() => {
            copy_to_second(&file, Path::new(folder), keep, state)
        }
        _ => None,
    };
    *lock(&state.backup_warning)? = warning.clone();
    tracing::info!(?file, reason, "backup made");
    Ok(BackupDone {
        file: file.to_string_lossy().into_owned(),
        warning,
    })
}

/// The timer's backup: only when something changed since the last one (BAK-01).
pub fn backup_if_changed(state: &AppState) -> Result<Option<BackupDone>, AppError> {
    if state.health != DbHealth::Ok {
        return Ok(None);
    }
    let since = *lock(&state.backed_up_at_marker)?;
    if !db::has_changed_since(&state.db, since) {
        return Ok(None);
    }
    make_backup(state, "auto").map(Some)
}

/// Every backup in `folder`, newest first, with what the restore list shows.
pub fn list_backups(folder: &Path) -> Result<Vec<BackupRow>, AppError> {
    let mut rows = Vec::new();
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Ok(rows);
    };
    let all = migrations::all();
    for entry in entries.flatten() {
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let Some((taken_at, reason)) = read_name(&file_name) else {
            continue;
        };
        let file = entry.path();
        let size_bytes = entry.metadata().map(|data| data.len()).unwrap_or(0);
        let usable = db::verify(&file, &all).is_ok();
        let summary = if usable { sales_summary(&file) } else { None };
        rows.push(BackupRow {
            file: file.to_string_lossy().into_owned(),
            file_name,
            taken_at,
            reason,
            size_bytes,
            usable,
            sale_count: summary.as_ref().map_or(0, |found| found.sale_count),
            last_invoice_no: summary.and_then(|found| found.last_invoice_no),
        });
    }
    rows.sort_by(|a, b| b.file_name.cmp(&a.file_name));
    Ok(rows)
}

/// Checks the file, backs up the current database (`pre-restore`) and asks for
/// the restore at the next start. The caller restarts the app (BAK-05).
pub fn request_restore(
    state: &AppState,
    file: &Path,
    by: Option<Id>,
    approved_by: Option<Id>,
) -> Result<(), AppError> {
    db::verify(file, &migrations::all())?;
    if state.health == DbHealth::Ok {
        make_backup(state, "pre-restore")?;
    }
    ask_for_restore(
        &state.paths,
        &RestoreRequest {
            file: file.to_path_buf(),
            user_id: by,
            approved_by,
        },
    )
}

/// The backup's file name. A second backup in the same second gets "-2", "-3", …
/// after its reason, because `backup_to` never overwrites a file.
fn free_file_name(folder: &Path, state: &AppState, reason: &str) -> PathBuf {
    let mut file = folder.join(db::backup_file_name(state.clock.as_ref(), reason));
    let mut number = 2;
    while file.exists() {
        let numbered = format!("{reason}-{number}");
        file = folder.join(db::backup_file_name(state.clock.as_ref(), &numbered));
        number += 1;
    }
    file
}

/// Reads "pos-20261005-143000-auto.sqlite" as ("2026-10-05 14:30:00", "auto").
fn read_name(name: &str) -> Option<(String, String)> {
    let rest = name.strip_prefix("pos-")?.strip_suffix(".sqlite")?;
    let (date, rest) = rest.split_once('-')?;
    let (time, reason) = rest.split_once('-')?;
    let digits = |text: &str, length: usize| {
        text.len() == length && text.bytes().all(|b| b.is_ascii_digit())
    };
    if !digits(date, 8) || !digits(time, 6) || reason.is_empty() {
        return None;
    }
    let taken_at = format!(
        "{}-{}-{} {}:{}:{}",
        &date[0..4],
        &date[4..6],
        &date[6..8],
        &time[0..2],
        &time[2..4],
        &time[4..6]
    );
    Some((taken_at, reason.to_string()))
}

fn sales_summary(file: &Path) -> Option<sales::SalesSummary> {
    let conn = db::open_read_only(file).ok()?;
    sales::sales_summary(&conn).ok()
}

fn copy_to_second(file: &Path, folder: &Path, keep: usize, state: &AppState) -> Option<String> {
    let copied = create_folder(folder).and_then(|()| {
        let target = folder.join(file.file_name().unwrap_or_default());
        std::fs::copy(file, target)
            .map(|_| ())
            .map_err(|error| AppError::internal(error.to_string()))
    });
    match copied {
        Ok(()) => {
            prune(folder, keep, state);
            None
        }
        Err(error) => {
            tracing::warn!(?folder, %error, "the second backup folder could not be used");
            Some("backup_second_folder_failed".into())
        }
    }
}

/// Deletes old backups. A failure is only logged: the backup itself was made.
fn prune(folder: &Path, keep: usize, state: &AppState) {
    if let Err(error) = db::prune_backups(folder, keep, KEEP_DAILY_DAYS, state.clock.as_ref()) {
        tracing::warn!(?folder, %error, "old backups could not be deleted");
    }
}

fn create_folder(folder: &Path) -> Result<(), AppError> {
    std::fs::create_dir_all(folder)
        .map_err(|error| AppError::internal(format!("could not make {folder:?}: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bak_05_backup_names_give_time_and_reason() {
        assert_eq!(
            read_name("pos-20261005-143000-pre-restore.sqlite"),
            Some(("2026-10-05 14:30:00".into(), "pre-restore".into()))
        );
        assert_eq!(read_name("notes.txt"), None);
        assert_eq!(read_name("pos-2026-143000-auto.sqlite"), None);
        assert_eq!(read_name("pos-20261005-143000-.sqlite"), None);
    }
}
