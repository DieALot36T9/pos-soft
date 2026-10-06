//! What every command shares: the database, the session, the clock and the paths.

use std::sync::{Arc, Mutex, MutexGuard};

use db::Db;
use pos_core::{AppError, Clock};

use crate::logging::LogSwitch;
use crate::paths::AppPaths;
use crate::session::{Approval, Session};

/// Whether the database passed the startup check (rule BAK-06).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbHealth {
    Ok,
    /// `quick_check` failed or the file isn't a database. Only the recovery screen works.
    Damaged,
}

/// Made once at startup. Tauri keeps it in its state.
pub struct AppState {
    pub paths: AppPaths,
    pub clock: Arc<dyn Clock>,
    pub db: Db,
    pub health: DbHealth,
    /// The signed-in user and whether the till is locked. Only in memory:
    /// restarting the app signs everyone out.
    pub session: Mutex<Option<Session>>,
    /// Admin approvals not used yet: single-use tokens, each good for one
    /// permission for 60 seconds (USR-05). Only in memory.
    pub approvals: Mutex<Vec<Approval>>,
    /// The database's change marker at the last backup, so the timer can skip a backup.
    pub backed_up_at_marker: Mutex<u64>,
    /// The last backup problem to show in the warning banner, as an error code.
    pub backup_warning: Mutex<Option<String>>,
    /// The detailed-logging switch. `None` in tests, which don't start logging.
    pub log: Option<LogSwitch>,
}

impl AppState {
    /// The signed-in user, if any.
    pub fn session(&self) -> Result<MutexGuard<'_, Option<Session>>, AppError> {
        lock(&self.session)
    }
}

/// Locks a mutex. A panic in another thread while it held the lock is a bug.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>, AppError> {
    mutex
        .lock()
        .map_err(|_| AppError::internal("a lock was left broken by a panic"))
}
