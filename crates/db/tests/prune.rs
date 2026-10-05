//! Contract tests for backup file names and pruning old backups (BAK-01), with a fixed clock.

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use db::{backup_file_name, prune_backups};
use pos_core::{FixedClock, Id, Timestamp};

/// Dhaka is 6 hours ahead of UTC.
const DHAKA: i32 = 360;

/// Builds a UTC timestamp from "YYYY-MM-DD HH:MM:SS".
fn utc(text: &str) -> Timestamp {
    let moment = chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S").unwrap();
    Timestamp(moment.and_utc().timestamp_millis())
}

struct Folder(PathBuf);

impl Folder {
    fn new() -> Folder {
        let path = std::env::temp_dir().join(format!("pos-db-prune-{}", Id::new()));
        std::fs::create_dir_all(&path).unwrap();
        Folder(path)
    }

    fn touch(&self, name: &str) {
        std::fs::write(self.0.join(name), b"backup").unwrap();
    }

    fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(&self.0)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn bak_01_backup_file_name_uses_local_time_and_reason() {
    // 22:05:09 UTC on 3 October is 04:05:09 local time on 4 October in Dhaka.
    let clock = FixedClock::new(utc("2026-10-03 22:05:09"), DHAKA);
    assert_eq!(
        backup_file_name(&clock, "timer"),
        "pos-20261004-040509-timer.sqlite"
    );
}

#[test]
fn bak_01_backup_file_name_keeps_only_safe_letters_in_the_reason() {
    let clock = FixedClock::new(utc("2026-10-04 00:00:00"), 0);
    assert_eq!(
        backup_file_name(&clock, "Pre Restore/../x"),
        "pos-20261004-000000-pre-restore----x.sqlite"
    );
}

#[test]
fn bak_01_prune_keeps_the_newest_n() {
    let folder = Folder::new();
    for name in [
        "pos-20261004-090000-timer.sqlite",
        "pos-20261004-100000-timer.sqlite",
        "pos-20261004-110000-timer.sqlite",
        "pos-20261004-120000-manual.sqlite",
    ] {
        folder.touch(name);
    }
    let clock = FixedClock::new(utc("2026-10-04 07:00:00"), DHAKA); // 13:00 local

    // No daily backups kept, so only the newest 2 stay.
    let deleted = prune_backups(&folder.0, 2, 0, &clock).unwrap();

    assert_eq!(
        folder.names(),
        vec![
            "pos-20261004-110000-timer.sqlite",
            "pos-20261004-120000-manual.sqlite",
        ]
    );
    assert_eq!(deleted.len(), 2);
}

#[test]
fn bak_01_prune_keeps_the_last_backup_of_each_day_for_30_days() {
    let folder = Folder::new();
    for name in [
        // Today (4 October): the newest 2 are kept by "keep last".
        "pos-20261004-080000-timer.sqlite",
        "pos-20261004-090000-timer.sqlite",
        "pos-20261004-100000-timer.sqlite",
        // Yesterday: only the last one of the day stays.
        "pos-20261003-090000-timer.sqlite",
        "pos-20261003-213000-close.sqlite",
        // 29 days ago (5 September): still inside the 30 days.
        "pos-20260905-100000-timer.sqlite",
        "pos-20260905-200000-close.sqlite",
        // 30 days ago (4 September): too old.
        "pos-20260904-200000-close.sqlite",
    ] {
        folder.touch(name);
    }
    let clock = FixedClock::new(utc("2026-10-04 05:00:00"), DHAKA); // 11:00 local

    prune_backups(&folder.0, 2, 30, &clock).unwrap();

    assert_eq!(
        folder.names(),
        vec![
            "pos-20260905-200000-close.sqlite",
            "pos-20261003-213000-close.sqlite",
            "pos-20261004-090000-timer.sqlite",
            "pos-20261004-100000-timer.sqlite",
        ]
    );
}

#[test]
fn bak_01_prune_leaves_other_files_alone() {
    let folder = Folder::new();
    for name in [
        "pos-20200101-000000-timer.sqlite",
        "notes.txt",
        "pos.sqlite",
        "pos-2020-bad-name.sqlite",
        "pos-20200101-000000-timer.sqlite-journal",
    ] {
        folder.touch(name);
    }
    let clock = FixedClock::new(utc("2026-10-04 05:00:00"), DHAKA);

    prune_backups(&folder.0, 0, 30, &clock).unwrap();

    assert_eq!(
        folder.names(),
        vec![
            "notes.txt",
            "pos-2020-bad-name.sqlite",
            "pos-20200101-000000-timer.sqlite-journal",
            "pos.sqlite",
        ]
    );
}

#[test]
fn bak_01_prune_of_a_missing_folder_is_an_error() {
    let clock = FixedClock::new(utc("2026-10-04 05:00:00"), DHAKA);
    let missing = std::env::temp_dir().join(format!("pos-db-missing-{}", Id::new()));
    assert!(prune_backups(&missing, 50, 30, &clock).is_err());
}
