//! Helpers for the app's workflow tests: a fresh app in a temporary data folder,
//! a fixed clock, and a shop that has finished setup.

// Each test file uses only some of these helpers.
#![allow(dead_code)]

use std::path::Path;
use std::sync::Arc;

use app::paths::AppPaths;
use app::state::AppState;
use app::workflows::setup::{self, AdminStep, MoneyStep, TaxStep, TillStep};
use app::workflows::tax_rates::{self, RateInput};
use pos_core::{
    Bps, Clock, CurrencyFormat, FixedClock, Money, RoundingAppliesTo, RoundingMode, RoundingRule,
    TaxMode, Timestamp,
};
use settings::{BackupSettings, Setting, ShopProfile};
use tax::NewRate;

/// 2026-10-05 08:00 UTC, 14:00 in Dhaka.
pub const START_MS: i64 = 1_791_187_200_000;

pub const ADMIN_PASSWORD: &str = "shop-owner-1";

/// Starts the app on this data folder, like opening it on a shop PC.
pub fn start(folder: &Path) -> AppState {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock::new(Timestamp(START_MS), 360));
    app::startup::start(AppPaths::new(folder), clock).expect("the app starts")
}

pub fn shop(name: &str) -> ShopProfile {
    ShopProfile {
        name: name.into(),
        ..ShopProfile::default_value()
    }
}

pub fn money() -> MoneyStep {
    MoneyStep {
        currency: CurrencyFormat::default_value(),
        rounding: RoundingRule {
            increment_minor: Money(100),
            mode: RoundingMode::Nearest,
            applies_to: RoundingAppliesTo::CashOnly,
        },
    }
}

pub fn rate(name: &str, bps: u32) -> RateInput {
    RateInput {
        id: None,
        rate: NewRate {
            name: name.into(),
            rate_bps: Bps(bps),
            components: Vec::new(),
            sort: 0,
        },
    }
}

pub fn admin() -> AdminStep {
    AdminStep {
        username: "owner".into(),
        display_name: "Shop Owner".into(),
        password: ADMIN_PASSWORD.into(),
        password_again: ADMIN_PASSWORD.into(),
    }
}

/// Runs every required wizard step (hardware skipped), as a shop owner would.
pub fn finish_setup(state: &AppState) {
    setup::save_shop(state, &shop("Rahim Store")).expect("shop");
    setup::save_money(state, &money()).expect("money");
    tax_rates::save(state, rate("Standard 15%", 1500), None).expect("rate");
    let tax = TaxStep {
        label: "VAT".into(),
        mode: TaxMode::Inclusive,
        default_rate_id: None,
    };
    setup::save_tax(state, &tax).expect("tax");
    setup::save_admin(state, admin()).expect("admin");
    let till = TillStep {
        till_code: "t1".into(),
        business_day_cutoff_hour: 4,
    };
    setup::save_till(state, &till).expect("till");
    setup::save_backup(state, &BackupSettings::default_value()).expect("backup");
    let status = setup::finish(state).expect("finish");
    assert!(status.is_complete);
}

/// How many audit events have this action.
pub fn audit_count(state: &AppState, action: &str) -> i64 {
    state
        .db
        .read(|conn| {
            Ok(conn
                .query_row(
                    "SELECT COUNT(*) FROM audit_events WHERE action = ?1",
                    [action],
                    |row| row.get(0),
                )
                .expect("audit count"))
        })
        .expect("read")
}
