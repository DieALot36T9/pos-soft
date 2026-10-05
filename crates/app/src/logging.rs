//! Log files (rule BAK-07): warnings and errors always, information and debug
//! lines only while "detailed logging" is on. One file a day, the last 14 kept.

use std::path::Path;

use pos_core::AppError;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{Builder, Rotation};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Registry, fmt, reload};

/// Log files kept, one per day.
const KEEP_DAYS: usize = 14;

/// Turns detailed logging on and off while the app runs, without a restart.
pub struct LogSwitch {
    level: reload::Handle<LevelFilter, Registry>,
    // Writing happens on a background thread; dropping this flushes and stops it.
    _writer: WorkerGuard,
}

/// The lowest level written to the file.
pub fn level_for(detailed: bool) -> LevelFilter {
    if detailed {
        LevelFilter::DEBUG
    } else {
        LevelFilter::WARN
    }
}

/// Starts logging into `folder` (`pos.YYYY-MM-DD.log`). Call once, at startup.
pub fn start_logging(folder: &Path, detailed: bool) -> Result<LogSwitch, AppError> {
    let files = Builder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix("pos")
        .filename_suffix("log")
        .max_log_files(KEEP_DAYS)
        .build(folder)
        .map_err(|error| AppError::internal(format!("could not open the log folder: {error}")))?;
    let (writer, guard) = tracing_appender::non_blocking(files);
    let (level, handle) = reload::Layer::new(level_for(detailed));
    tracing_subscriber::registry()
        .with(level)
        .with(fmt::layer().with_writer(writer).with_ansi(false))
        .try_init()
        .map_err(|error| AppError::internal(format!("logging was already started: {error}")))?;
    log_panics();
    Ok(LogSwitch {
        level: handle,
        _writer: guard,
    })
}

impl LogSwitch {
    /// Switches detailed logging on or off right away.
    pub fn set_detailed(&self, detailed: bool) -> Result<(), AppError> {
        self.level
            .reload(level_for(detailed))
            .map_err(|error| AppError::internal(format!("could not change the log level: {error}")))
    }
}

/// Writes every panic to the log before the usual panic message.
fn log_panics() {
    let usual = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(panic = %info, backtrace = %std::backtrace::Backtrace::force_capture());
        usual(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bak_07_errors_and_warnings_are_always_written() {
        assert_eq!(level_for(false), LevelFilter::WARN);
        assert_eq!(level_for(true), LevelFilter::DEBUG);
    }
}
