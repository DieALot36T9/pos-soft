//! Time: timestamps, business dates and the clock (ADR-016).

use std::fmt;
use std::sync::atomic::{AtomicI32, AtomicI64, Ordering};

use chrono::{DateTime, Local, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::AppError;

/// A moment in time: milliseconds since 1970-01-01 UTC. Stored in `*_at` columns.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS,
)]
#[serde(transparent)]
#[ts(export)]
pub struct Timestamp(pub i64);

/// The trading day a sale belongs to. Sent and stored as "YYYY-MM-DD".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(transparent)]
#[ts(export)]
pub struct BusinessDate(pub NaiveDate);

impl BusinessDate {
    /// Reads a date written as "YYYY-MM-DD".
    pub fn parse(text: &str) -> Result<BusinessDate, AppError> {
        NaiveDate::parse_from_str(text, "%Y-%m-%d")
            .map(BusinessDate)
            .map_err(|_| AppError::validation("date_invalid"))
    }
}

impl fmt::Display for BusinessDate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0.format("%Y-%m-%d"))
    }
}

/// Where "now" comes from. The app uses `SystemClock`; tests use `FixedClock`.
pub trait Clock: Send + Sync {
    /// The current time.
    fn now(&self) -> Timestamp;
    /// Minutes between local time and UTC right now. Dhaka is +360.
    fn local_offset_minutes(&self) -> i32;
}

/// The computer's real clock and time zone.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp(Utc::now().timestamp_millis())
    }

    fn local_offset_minutes(&self) -> i32 {
        Local::now().offset().local_minus_utc() / 60
    }
}

/// A clock for tests: it shows the time you set until you change it.
#[derive(Debug, Default)]
pub struct FixedClock {
    now_ms: AtomicI64,
    offset_minutes: AtomicI32,
}

impl FixedClock {
    /// A clock stopped at `now`, in a time zone `offset_minutes` from UTC.
    pub fn new(now: Timestamp, offset_minutes: i32) -> FixedClock {
        FixedClock {
            now_ms: AtomicI64::new(now.0),
            offset_minutes: AtomicI32::new(offset_minutes),
        }
    }

    /// Moves the clock to a new time.
    pub fn set(&self, now: Timestamp) {
        self.now_ms.store(now.0, Ordering::SeqCst);
    }

    /// Moves the clock forward by some milliseconds.
    pub fn advance_ms(&self, milliseconds: i64) {
        self.now_ms.fetch_add(milliseconds, Ordering::SeqCst);
    }

    /// Changes the time zone offset, for example to test a daylight-saving change.
    pub fn set_offset_minutes(&self, offset_minutes: i32) {
        self.offset_minutes.store(offset_minutes, Ordering::SeqCst);
    }
}

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        Timestamp(self.now_ms.load(Ordering::SeqCst))
    }

    fn local_offset_minutes(&self) -> i32 {
        self.offset_minutes.load(Ordering::SeqCst)
    }
}

/// The trading day for a moment: the local date of (now − cutoff hours).
/// With cutoff 4, a sale at 00:30 local time belongs to the day before.
/// `offset_minutes` is the local offset at that moment, so daylight-saving changes work.
pub fn business_date(
    now: Timestamp,
    offset_minutes: i32,
    cutoff_hour: u8,
) -> Result<BusinessDate, AppError> {
    let shift_ms = i64::from(offset_minutes) * 60_000 - i64::from(cutoff_hour) * 3_600_000;
    let local_ms = now
        .0
        .checked_add(shift_ms)
        .ok_or_else(|| AppError::internal(format!("business_date: time {} overflowed", now.0)))?;
    let moment = DateTime::from_timestamp_millis(local_ms).ok_or_else(|| {
        AppError::internal(format!("business_date: time {} is out of range", now.0))
    })?;
    Ok(BusinessDate(moment.date_naive()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a UTC timestamp from "YYYY-MM-DD HH:MM".
    fn utc(text: &str) -> Timestamp {
        let moment = chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M").unwrap();
        Timestamp(moment.and_utc().timestamp_millis())
    }

    fn date(text: &str) -> BusinessDate {
        BusinessDate::parse(text).unwrap()
    }

    #[test]
    fn sale_after_midnight_belongs_to_the_day_before() {
        // Dhaka (+6:00): 00:30 local on 2 March is 18:30 UTC on 1 March.
        let now = utc("2026-03-01 18:30");
        assert_eq!(business_date(now, 360, 4).unwrap(), date("2026-03-01"));
    }

    #[test]
    fn sale_at_the_cutoff_starts_the_new_day() {
        // 04:00 local on 2 March in Dhaka.
        let now = utc("2026-03-01 22:00");
        assert_eq!(business_date(now, 360, 4).unwrap(), date("2026-03-02"));
        // One minute earlier is still 1 March.
        assert_eq!(
            business_date(utc("2026-03-01 21:59"), 360, 4).unwrap(),
            date("2026-03-01")
        );
    }

    #[test]
    fn cutoff_zero_uses_the_plain_local_date() {
        assert_eq!(
            business_date(utc("2026-03-01 18:30"), 360, 0).unwrap(),
            date("2026-03-02")
        );
    }

    #[test]
    fn daylight_saving_change_uses_the_offset_at_that_moment() {
        // London clocks go forward at 01:00 UTC on 29 March 2026 (offset 0 → +60).
        // 03:30 UTC is 04:30 summer time: past the 4 o'clock cutoff, so it's the 29th.
        assert_eq!(
            business_date(utc("2026-03-29 03:30"), 60, 4).unwrap(),
            date("2026-03-29")
        );
        // 03:30 UTC the day before was 03:30 winter time: before the cutoff, so the 27th.
        assert_eq!(
            business_date(utc("2026-03-28 03:30"), 0, 4).unwrap(),
            date("2026-03-27")
        );
    }

    #[test]
    fn negative_offsets_work_too() {
        // New York (−5:00): 23:00 local on 1 March is 04:00 UTC on 2 March.
        assert_eq!(
            business_date(utc("2026-03-02 04:00"), -300, 4).unwrap(),
            date("2026-03-01")
        );
    }

    #[test]
    fn fixed_clock_moves_only_when_told() {
        let clock = FixedClock::new(Timestamp(1_000), 360);
        assert_eq!(clock.now(), Timestamp(1_000));
        clock.advance_ms(500);
        assert_eq!(clock.now(), Timestamp(1_500));
        clock.set_offset_minutes(60);
        assert_eq!(clock.local_offset_minutes(), 60);
    }

    #[test]
    fn business_date_is_sent_as_text() {
        assert_eq!(
            serde_json::to_string(&date("2026-03-02")).unwrap(),
            r#""2026-03-02""#
        );
        assert_eq!(date("2026-03-02").to_string(), "2026-03-02");
    }
}
