//! Helpers for the app's workflow tests: a fresh app in a temporary data folder,
//! a fixed clock, and a shop that has finished setup.

// Each test file uses only some of these helpers.
#![allow(dead_code)]

use std::path::Path;
use std::sync::Arc;

use app::paths::AppPaths;
use app::session::LoginResult;
use app::state::AppState;
use app::workflows::setup::{self, AdminStep, MoneyStep, TaxStep, TillStep};
use app::workflows::tax_rates::{self, RateInput};
use auth::NewUser;
use pos_core::{
    Bps, Clock, CurrencyFormat, FixedClock, Id, Money, RoundingAppliesTo, RoundingMode,
    RoundingRule, Secret, TaxMode, Timestamp,
};
use settings::{BackupSettings, Setting, ShopProfile};
use tax::NewRate;

/// The devices every test shares: fonts, the Preview printer and the fake scale.
/// Each test runs in its own process, so tests never see each other's printouts.
pub fn devices() -> &'static app::devices::Devices {
    static DEVICES: std::sync::OnceLock<app::devices::Devices> = std::sync::OnceLock::new();
    DEVICES.get_or_init(|| app::devices::Devices::new(None))
}

/// 2026-10-05 08:00 UTC, 14:00 in Dhaka.
pub const START_MS: i64 = 1_791_187_200_000;

pub const ADMIN_PASSWORD: &str = "shop-owner-1";

/// Starts the app on this data folder, like opening it on a shop PC.
pub fn start(folder: &Path) -> AppState {
    start_with_clock(folder).0
}

/// Like `start`, and gives the clock so a test can move time on.
pub fn start_with_clock(folder: &Path) -> (AppState, Arc<FixedClock>) {
    let clock = Arc::new(FixedClock::new(Timestamp(START_MS), 360));
    let shared: Arc<dyn Clock> = clock.clone();
    let state = app::startup::start(AppPaths::new(folder), shared).expect("the app starts");
    (state, clock)
}

/// The id of the active user with this username.
pub fn user_id(state: &AppState, username: &str) -> Id {
    app::session::login_tiles(state)
        .expect("tiles")
        .into_iter()
        .find(|tile| tile.username.eq_ignore_ascii_case(username))
        .map(|tile| tile.id)
        .expect("the user has a tile")
}

/// Logs in with a PIN or password and checks it worked.
pub fn sign_in(state: &AppState, username: &str, secret: &str) {
    let result =
        app::session::login(state, &user_id(state, username), secret.into()).expect("login runs");
    assert!(matches!(result, LoginResult::LoggedIn { .. }), "{result:?}");
}

/// Adds a user straight through `auth`, as if made on the users screen.
pub fn add_user(
    state: &AppState,
    username: &str,
    role_id: &str,
    password: Option<&str>,
    pin: Option<&str>,
) -> Id {
    let now = state.clock.now();
    state
        .db
        .write(|tx| {
            let user = NewUser {
                username: username.into(),
                display_name: format!("{username} name"),
                role_id: Id::parse(role_id)?,
                password: password.map(|text| Secret::new(text.to_string())),
                pin: pin.map(|text| Secret::new(text.to_string())),
            };
            Ok(auth::create_user(tx, user, now)?)
        })
        .expect("user added")
}

/// A cashier "rina" with PIN 1234.
pub fn add_cashier(state: &AppState) -> Id {
    add_user(state, "rina", auth::CASHIER_ROLE_ID, None, Some("1234"))
}

/// The newest audit event with this action: (user_id, approved_by, details).
pub fn last_audit(state: &AppState, action: &str) -> (Option<String>, Option<String>, String) {
    state
        .db
        .read(|conn| {
            Ok(conn
                .query_row(
                    "SELECT user_id, approved_by, details_json FROM audit_events
                     WHERE action = ?1 ORDER BY at DESC, rowid DESC LIMIT 1",
                    [action],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .expect("an audit event"))
        })
        .expect("read")
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
/// Returns the recovery code shown in the admin step, and leaves the admin signed in.
pub fn finish_setup(state: &AppState) -> String {
    setup::save_shop(state, &shop("Rahim Store")).expect("shop");
    setup::save_money(state, &money()).expect("money");
    tax_rates::save(state, rate("Standard 15%", 1500), None).expect("rate");
    let tax = TaxStep {
        label: "VAT".into(),
        mode: TaxMode::Inclusive,
        default_rate_id: None,
    };
    setup::save_tax(state, &tax).expect("tax");
    let created = setup::save_admin(state, admin()).expect("admin");
    let till = TillStep {
        till_code: "t1".into(),
        business_day_cutoff_hour: 4,
    };
    setup::save_till(state, &till).expect("till");
    setup::save_backup(state, &BackupSettings::default_value()).expect("backup");
    let status = setup::finish(state).expect("finish");
    assert!(status.is_complete);
    created.recovery_code
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

/// A product for the till: price and unit as given, cost 60% of the price, one barcode.
/// `scale_code` is the item code inside scale stickers (CHK-08).
pub fn till_product(
    state: &AppState,
    name: &str,
    code: &str,
    price_minor: i64,
    unit: &str,
    weighed: bool,
    scale_code: Option<&str>,
) -> Id {
    let input = app::workflows::products::ProductInput {
        id: None,
        kind: catalog::ProductKind::Standard,
        name: name.into(),
        sku: String::new(),
        category_id: None,
        brand_id: None,
        unit_id: Id::parse(unit).expect("unit id"),
        sale_price: Money(price_minor),
        cost_price: Money(price_minor * 6 / 10),
        tax_rate_id: None,
        is_weighed: weighed,
        scale_item_code: scale_code.map(str::to_string),
        track_stock: true,
        track_expiry: false,
        preferred_supplier_id: None,
        description: None,
        barcodes: if code.is_empty() {
            Vec::new()
        } else {
            vec![code.into()]
        },
        reorder: None,
    };
    app::commands::catalog::product_save(state, input).expect("product saved")
}

/// Puts stock in with the "opening" reason, as the stock screen would.
pub fn stock_in(state: &AppState, product: &Id, qty: pos_core::Qty) {
    let input = app::workflows::stock::AdjustInput {
        product_id: product.clone(),
        qty_delta: qty,
        reason_code: "opening".into(),
        note: None,
        batch: app::workflows::stock::AdjustBatch::Auto,
    };
    app::commands::inventory::stock_adjust(state, &input).expect("stock in");
}

/// Opens a shift for the signed-in user with this float and no count.
pub fn open_shift(state: &AppState, float_minor: i64) {
    app::commands::shifts::shift_open(state, Money(float_minor), &[]).expect("shift open");
}

/// The id of the system cash payment method.
pub fn cash_method(state: &AppState) -> Id {
    state
        .db
        .read(|conn| {
            Ok(sales::system_payment_method(
                conn,
                sales::PaymentKind::Cash,
            )?)
        })
        .expect("cash method")
        .id
}

/// Cash handed over at the payment dialog.
pub fn cash(state: &AppState, amount_minor: i64) -> app::workflows::complete_sale::PaymentEntry {
    app::workflows::complete_sale::PaymentEntry {
        method_id: cash_method(state),
        amount: Money(amount_minor),
        reference: None,
    }
}

/// Compares a printout with its approved picture in `tests/golden/`, dot for dot, or
/// saves it when UPDATE_GOLDEN=1. A changed picture is saved as `<name>.new.png` (not
/// committed) to compare; it needs the product owner's approval.
pub fn check_golden(name: &str, picture: &pos_core::Bitmap1) {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden");
    let png = printing::to_png(picture).expect("the picture saves as PNG");
    let approved_path = folder.join(format!("{name}.png"));
    let new_path = folder.join(format!("{name}.new.png"));
    if std::env::var("UPDATE_GOLDEN").is_ok_and(|value| value == "1") {
        std::fs::write(&approved_path, &png).expect("the approved picture saves");
        let _ = std::fs::remove_file(&new_path);
        return;
    }
    let Ok(approved_png) = std::fs::read(&approved_path) else {
        std::fs::write(&new_path, &png).expect("the new picture saves");
        panic!("{name}: no approved picture yet. The new one is at {new_path:?}");
    };
    if &picture_from_png(&approved_png) != picture {
        std::fs::write(&new_path, &png).expect("the new picture saves");
        panic!("{name}: the picture changed. Compare {new_path:?} with {approved_path:?}");
    }
    let _ = std::fs::remove_file(&new_path);
}

/// Reads a PNG back into a 1-bit picture (black = 0 in the PNG).
pub fn picture_from_png(png: &[u8]) -> pos_core::Bitmap1 {
    let gray = image::load_from_memory(png)
        .expect("the picture is a PNG")
        .to_luma8();
    let mut picture = pos_core::Bitmap1::new(gray.width(), gray.height());
    let bytes_per_row = gray.width().div_ceil(8);
    for (x, y, pixel) in gray.enumerate_pixels() {
        if pixel.0[0] < 128 {
            picture.bits[(y * bytes_per_row + x / 8) as usize] |= 0x80 >> (x % 8);
        }
    }
    picture
}
