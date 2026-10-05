//! Starting the app (rules BAK-01, BAK-05, BAK-06): finish a restore asked for
//! before the restart, open the database, check it, back it up before an upgrade,
//! and bring its tables up to date.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use db::Db;
use pos_core::{AppError, Clock, Id};
use serde::{Deserialize, Serialize};

use crate::audit_actions::BACKUP_RESTORE;
use crate::audit_log;
use crate::migrations;
use crate::paths::AppPaths;
use crate::state::{AppState, DbHealth};

/// What the restore marker file holds: which backup to put back, and who asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreRequest {
    pub file: PathBuf,
    pub user_id: Option<Id>,
}

/// Opens everything the app needs. A damaged database doesn't stop the start:
/// the state says `Damaged` and only the recovery screen can be used (BAK-06).
pub fn start(paths: AppPaths, clock: Arc<dyn Clock>) -> Result<AppState, AppError> {
    paths.create_folders()?;
    let restored = apply_pending_restore(&paths)?;
    let (database, health) = open_checked(&paths.database())?;
    if health == DbHealth::Ok {
        back_up_before_upgrade(&database, &paths, clock.as_ref())?;
        db::migrate(&database, &migrations::all())?;
        if let Some(request) = &restored {
            record_restore(&database, clock.as_ref(), request)?;
        }
    }
    let marker = db::change_marker(&database);
    Ok(AppState {
        paths,
        clock,
        db: database,
        health,
        session: Mutex::new(None),
        backed_up_at_marker: Mutex::new(marker),
        backup_warning: Mutex::new(None),
        log: None,
    })
}

/// Asks for `file` to be restored at the next start (the caller then restarts the app).
pub fn ask_for_restore(paths: &AppPaths, request: &RestoreRequest) -> Result<(), AppError> {
    let text = serde_json::to_string(request)
        .map_err(|error| AppError::internal(format!("restore request: {error}")))?;
    std::fs::write(paths.restore_marker(), text)
        .map_err(|error| AppError::internal(format!("could not save the restore request: {error}")))
}

/// Puts the asked-for backup in place of the database, before it is opened.
fn apply_pending_restore(paths: &AppPaths) -> Result<Option<RestoreRequest>, AppError> {
    let marker = paths.restore_marker();
    let Ok(text) = std::fs::read_to_string(&marker) else {
        return Ok(None);
    };
    // The marker goes first, so a restore that fails can't repeat at every start.
    remove(&marker)?;
    let request: RestoreRequest = serde_json::from_str(&text)
        .map_err(|error| AppError::internal(format!("unreadable restore request: {error}")))?;
    let database = paths.database();
    for ending in ["", "-wal", "-shm"] {
        let file = PathBuf::from(format!("{}{ending}", database.display()));
        if file.exists() {
            remove(&file)?;
        }
    }
    std::fs::copy(&request.file, &database).map_err(|error| {
        AppError::internal(format!("could not copy {:?} back: {error}", request.file))
    })?;
    Ok(Some(request))
}

/// Opens the database and runs `quick_check`. A file SQLite can't open at all is
/// treated as damaged too, with an empty stand-in so the recovery screen still works.
fn open_checked(file: &Path) -> Result<(Db, DbHealth), AppError> {
    match Db::open(file) {
        Ok(database) => {
            let healthy = db::quick_check(&database).unwrap_or(false);
            let health = if healthy {
                DbHealth::Ok
            } else {
                DbHealth::Damaged
            };
            if !healthy {
                tracing::error!(?file, "the database failed its startup check");
            }
            Ok((database, health))
        }
        Err(error) => {
            tracing::error!(?file, %error, "the database could not be opened");
            Ok((Db::open_in_memory()?, DbHealth::Damaged))
        }
    }
}

/// Backs up an existing database before new migrations change it (BAK-01).
/// A new, empty database has nothing to back up.
fn back_up_before_upgrade(
    database: &Db,
    paths: &AppPaths,
    clock: &dyn Clock,
) -> Result<(), AppError> {
    let all = migrations::all();
    let waiting = db::pending(database, &all)?;
    let total: usize = all.iter().map(|module| module.migrations.len()).sum();
    if waiting.is_empty() || waiting.len() == total {
        return Ok(());
    }
    let file = paths
        .backups()
        .join(db::backup_file_name(clock, "pre-upgrade"));
    db::backup_to(database, &file)?;
    tracing::warn!(?file, "backed up before upgrading the database");
    Ok(())
}

/// Writes the restore into the audit log of the restored database (BAK-05).
fn record_restore(
    database: &Db,
    clock: &dyn Clock,
    request: &RestoreRequest,
) -> Result<(), AppError> {
    let file_name = request
        .file
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    database.write(|tx| {
        audit_log::record(
            tx,
            clock,
            BACKUP_RESTORE,
            serde_json::json!({ "file": file_name }),
            request.user_id.clone(),
        )
    })
}

fn remove(file: &Path) -> Result<(), AppError> {
    std::fs::remove_file(file)
        .map_err(|error| AppError::internal(format!("could not remove {file:?}: {error}")))
}
