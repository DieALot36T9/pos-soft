//! Background jobs: the backup timer (rule BAK-01).

use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::state::AppState;
use crate::workflows::backup;

/// How often the timer wakes up to see whether a backup is due.
const TICK: Duration = Duration::from_secs(30);

/// Starts the backup timer thread. Every `interval_min` minutes (read again each
/// time, so a change in Settings counts) it backs up if anything changed.
pub fn start_backup_timer(state: Arc<AppState>) {
    thread::spawn(move || {
        let mut last = Instant::now();
        loop {
            thread::sleep(TICK);
            let interval = backup::backup_settings(&state)
                .map(|options| Duration::from_secs(u64::from(options.interval_min) * 60))
                .unwrap_or(Duration::from_secs(3600));
            if last.elapsed() < interval {
                continue;
            }
            last = Instant::now();
            if let Err(error) = backup::backup_if_changed(&state) {
                tracing::error!(%error, "the timed backup failed");
            }
        }
    });
}

/// The backup made when the app closes (BAK-01). Errors are logged; closing goes on.
pub fn backup_on_close(state: &AppState) {
    if state.health != crate::state::DbHealth::Ok {
        return;
    }
    let setup_done = state
        .db
        .read(|conn| Ok(settings::setup_status(conn)?))
        .map(|status| status.is_complete)
        .unwrap_or(false);
    if !setup_done {
        return;
    }
    if let Err(error) = backup::make_backup(state, "close") {
        tracing::error!(%error, "the backup on close failed");
    }
}
