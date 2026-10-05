//! Backup file names and deleting old backups (BAK-01).
//! Names look like `pos-20261004-040509-timer.sqlite`: local date, local time, reason.

use std::path::{Path, PathBuf};

use pos_core::{BusinessDate, Clock, business_date};

use crate::DbError;

const MS_PER_DAY: i64 = 86_400_000;

/// The file name for a new backup made now, for example `pos-20261004-040509-timer.sqlite`.
/// The reason keeps only lower-case letters, digits and dashes; anything else becomes a dash.
pub fn backup_file_name(clock: &dyn Clock, reason: &str) -> String {
    let now = clock.now();
    let offset_minutes = clock.local_offset_minutes();
    // A cutoff of 0 gives the plain local date. It can only fail for times
    // thousands of years away, which a real clock never shows.
    let date = match business_date(now, offset_minutes, 0) {
        Ok(date) => date.to_string().replace('-', ""),
        Err(_) => "00000000".to_string(),
    };
    let local_ms = now
        .0
        .saturating_add(i64::from(offset_minutes).saturating_mul(60_000));
    let seconds_of_day = local_ms.rem_euclid(MS_PER_DAY) / 1000;
    let time = format!(
        "{:02}{:02}{:02}",
        seconds_of_day / 3600,
        seconds_of_day / 60 % 60,
        seconds_of_day % 60
    );
    format!("pos-{date}-{time}-{}.sqlite", safe_reason(reason))
}

/// Deletes old backups in `dir`. It keeps the newest `keep_last`, plus the last
/// backup of each day for `keep_daily_days` days (today counts as the first day).
/// Files that aren't named like a backup are never touched. Returns what it deleted.
pub fn prune_backups(
    dir: &Path,
    keep_last: usize,
    keep_daily_days: u32,
    clock: &dyn Clock,
) -> Result<Vec<PathBuf>, DbError> {
    let mut backups = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if let Some(backup) = read_backup_name(&name) {
            backups.push(backup);
        }
    }

    let today = business_date(clock.now(), clock.local_offset_minutes(), 0)
        .map_err(|error| DbError::Io(std::io::Error::other(error.to_string())))?;
    let mut deleted = Vec::new();
    for name in backups_to_delete(backups, keep_last, keep_daily_days, today) {
        let path = dir.join(name);
        std::fs::remove_file(&path)?;
        deleted.push(path);
    }
    Ok(deleted)
}

/// One backup file found in the folder.
struct BackupName {
    name: String,
    /// Date and time as "YYYYMMDDHHMMSS", so sorting the text sorts by time.
    stamp: String,
    date: BusinessDate,
}

/// Picks the backups to delete. Kept: the newest `keep_last`, and the newest
/// backup of each day that is less than `keep_daily_days` days old.
fn backups_to_delete(
    mut backups: Vec<BackupName>,
    keep_last: usize,
    keep_daily_days: u32,
    today: BusinessDate,
) -> Vec<String> {
    // Newest first.
    backups.sort_by(|a, b| b.stamp.cmp(&a.stamp).then_with(|| b.name.cmp(&a.name)));
    let mut days_seen: Vec<BusinessDate> = Vec::new();
    let mut to_delete = Vec::new();
    for (position, backup) in backups.into_iter().enumerate() {
        let newest_of_its_day = !days_seen.contains(&backup.date);
        if newest_of_its_day {
            days_seen.push(backup.date);
        }
        let days_old = (today.0 - backup.date.0).num_days();
        let kept_as_daily = newest_of_its_day && days_old < i64::from(keep_daily_days);
        if position >= keep_last && !kept_as_daily {
            to_delete.push(backup.name);
        }
    }
    to_delete
}

/// Reads a name like `pos-20261004-040509-timer.sqlite`. Anything else gives `None`.
fn read_backup_name(name: &str) -> Option<BackupName> {
    let middle = name.strip_prefix("pos-")?.strip_suffix(".sqlite")?;
    let date_digits = middle.get(0..8)?;
    let time_digits = middle.get(9..15)?;
    let reason = middle.get(16..)?;
    let all_digits = |text: &str| text.bytes().all(|byte| byte.is_ascii_digit());
    let separators_ok = middle.get(8..9)? == "-" && middle.get(15..16)? == "-";
    if !all_digits(date_digits) || !all_digits(time_digits) || !separators_ok || reason.is_empty() {
        return None;
    }
    let date_text = format!(
        "{}-{}-{}",
        &date_digits[0..4],
        &date_digits[4..6],
        &date_digits[6..8]
    );
    let date = BusinessDate::parse(&date_text).ok()?;
    Some(BackupName {
        name: name.to_string(),
        stamp: format!("{date_digits}{time_digits}"),
        date,
    })
}

fn safe_reason(reason: &str) -> String {
    let cleaned: String = reason
        .chars()
        .map(|letter| {
            let letter = letter.to_ascii_lowercase();
            if letter.is_ascii_lowercase() || letter.is_ascii_digit() || letter == '-' {
                letter
            } else {
                '-'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "backup".to_string()
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_backup_name() {
        let backup = read_backup_name("pos-20261004-040509-pre-restore.sqlite").unwrap();
        assert_eq!(backup.stamp, "20261004040509");
        assert_eq!(backup.date.to_string(), "2026-10-04");
    }

    #[test]
    fn refuses_names_that_are_not_backups() {
        for name in [
            "pos.sqlite",
            "pos-20261004-040509-.sqlite",
            "pos-20261304-040509-timer.sqlite", // month 13
            "pos-2026100a-040509-timer.sqlite",
            "pos-20261004_040509-timer.sqlite",
            "pos-20261004-040509-timer.sqlite-wal",
        ] {
            assert!(read_backup_name(name).is_none(), "{name}");
        }
    }

    #[test]
    fn empty_reason_becomes_backup() {
        assert_eq!(safe_reason(""), "backup");
    }
}
