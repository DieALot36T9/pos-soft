//! Power-cut test (NFR-R1, BAK-06): a child process keeps committing rows and is
//! killed at random moments. After each kill the database must pass `quick_check`
//! and still hold the last row the child said was committed.
//!
//! The child is this same test binary, started again with `POWER_CUT_DB` set.
//! Killing the process is the closest a test can get to pulling the plug.

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, SystemTime};

use db::{Db, migrate, quick_check};
use pos_core::{AppError, Id, Migration, ModuleMigrations};

const CHILD_TEST: &str = "power_cut_child_writer";
const ROUNDS: u32 = 8;
/// The child stops by itself after this many rows, in case the parent died.
const CHILD_ROW_LIMIT: i64 = 200_000;

const ROWS_V1: &[Migration] = &[Migration {
    version: 1,
    name: "init",
    sql: "CREATE TABLE powercut_rows (n INTEGER PRIMARY KEY, filler TEXT NOT NULL);",
}];

fn rows_module() -> ModuleMigrations {
    ModuleMigrations {
        module: "powercut",
        migrations: ROWS_V1,
    }
}

/// Runs only as the child: commits rows 1, 2, 3, … and prints each number after
/// its commit returns. Without `POWER_CUT_DB` it does nothing and passes.
#[test]
fn power_cut_child_writer() {
    let Ok(path) = std::env::var("POWER_CUT_DB") else {
        return;
    };
    let db = Db::open(Path::new(&path)).unwrap();
    migrate(&db, &[rows_module()]).unwrap();
    let mut next: i64 = db
        .read(|conn| {
            conn.query_row(
                "SELECT coalesce(max(n), 0) + 1 FROM powercut_rows",
                [],
                |row| row.get(0),
            )
            .map_err(|error| AppError::internal(error.to_string()))
        })
        .unwrap();
    let mut out = std::io::stdout().lock();
    while next < CHILD_ROW_LIMIT {
        db.write(|tx| {
            tx.execute(
                "INSERT INTO powercut_rows (n, filler) VALUES (:n, :filler)",
                rusqlite::named_params! { ":n": next, ":filler": "x".repeat(200) },
            )
            .map_err(|error| AppError::internal(error.to_string()))?;
            Ok(())
        })
        .unwrap();
        writeln!(out, "committed {next}").unwrap();
        out.flush().unwrap();
        next += 1;
    }
}

#[test]
fn nfr_r1_committed_rows_survive_a_killed_process() {
    let folder = std::env::temp_dir().join(format!("pos-db-power-{}", Id::new()));
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("pos.sqlite");

    for round in 0..ROUNDS {
        let last_committed = run_child_and_kill(&path, round);
        assert!(
            last_committed > 0,
            "round {round}: the child committed nothing"
        );

        let db = Db::open(&path).unwrap();
        assert!(
            quick_check(&db).unwrap(),
            "round {round}: quick_check failed"
        );
        let found: i64 = db
            .read(|conn| {
                conn.query_row(
                    "SELECT count(*) FROM powercut_rows WHERE n = :n",
                    rusqlite::named_params! { ":n": last_committed },
                    |row| row.get(0),
                )
                .map_err(|error| AppError::internal(error.to_string()))
            })
            .unwrap();
        assert_eq!(found, 1, "round {round}: row {last_committed} was lost");
    }

    std::fs::remove_dir_all(&folder).unwrap();
}

/// Starts the child, waits until it has committed something, kills it after a
/// random short wait, and returns the last row number it reported.
fn run_child_and_kill(path: &PathBuf, round: u32) -> i64 {
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([CHILD_TEST, "--exact", "--nocapture", "--test-threads=1"])
        .env("POWER_CUT_DB", path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    // Read the child's output on a thread, so the pipe never fills up.
    let stdout = child.stdout.take().unwrap();
    let (sender, receiver) = mpsc::channel::<i64>();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            let Some(number) = line.strip_prefix("committed ") else {
                continue;
            };
            if sender.send(number.trim().parse().unwrap()).is_err() {
                break;
            }
        }
    });

    let mut last_committed = receiver.recv_timeout(Duration::from_secs(60)).unwrap();
    std::thread::sleep(Duration::from_millis(random_millis(round)));
    child.kill().unwrap();
    child.wait().unwrap();
    reader.join().unwrap();
    while let Ok(number) = receiver.try_recv() {
        last_committed = number;
    }
    last_committed
}

/// A wait between 1 and 150 ms. No random crate is needed for this: the clock's
/// nanoseconds are different on every run.
fn random_millis(round: u32) -> u64 {
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|elapsed| elapsed.subsec_nanos())
        .unwrap_or(0);
    u64::from(nanos.wrapping_add(round.wrapping_mul(7919)) % 150) + 1
}
