//! A whole trading day at the till, driven at random (Module - app, "Scripted day";
//! milestones M6 and M9): open a shift, sell, hold, resume, cancel, move cash, return
//! and void hundreds of times, kill the app in the middle, then close the shift.
//! Afterwards every rule still holds: no completed sale is lost, invoice and return
//! numbers are gapless, payments add up to each sale, the drawer's expected cash
//! matches, and stock matches its ledger.

mod common;

use app::commands::checkout as till;
use app::commands::returns as return_commands;
use app::commands::shifts as shift_commands;
use app::state::AppState;
use app::workflows::checkout::MethodKind;
use app::workflows::complete_sale::PaymentEntry;
use app::workflows::payment_methods::PaymentMethodSave;
use app::workflows::returns::{ReturnInput, ReturnLineInput};
use app::workflows::shifts::{CashMoveInput, CashMoveKind, CountLine};
use common::*;
use pos_core::{AppError, Id, Money, Qty};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

const FLOAT: i64 = 200_000;
const STEPS: usize = 300;

/// Everything the test tracks on its own, to compare with what the app saved.
#[derive(Default)]
struct Expected {
    sales: u32,
    cash_kept: i64,
    cash_moves: i64,
    /// Every sale made: (id, invoice number, still open to a void).
    sold: Vec<(Id, String, bool)>,
    returns: u32,
}

fn prices() -> [(&'static str, i64); 5] {
    [
        ("A-1", 9_000),
        ("B-2", 4_550),
        ("C-3", 12_075),
        ("D-4", 999),
        ("E-5", 33_310),
    ]
}

#[test]
fn m6_a_random_day_keeps_every_rule() {
    let folder = tempfile::tempdir().expect("folder");
    let (mut state, clock) = start_with_clock(folder.path());
    finish_setup(&state);
    for (code, price) in prices() {
        let id = till_product(&state, code, code, price, catalog::UNIT_PIECE, false, None);
        stock_in(&state, &id, Qty(500_000));
    }
    let card = app::commands::payment_methods::payment_method_save(
        &state,
        &PaymentMethodSave {
            id: None,
            name: "Card".into(),
            kind: MethodKind::Card,
            requires_reference: false,
            active: true,
            sort: 5,
        },
    )
    .expect("card");
    open_shift(&state, FLOAT);

    let mut random = StdRng::seed_from_u64(6);
    let mut expected = Expected::default();
    for step in 0..STEPS {
        clock.advance_ms(30_000);
        if step == STEPS / 2 {
            // The app is killed and started again; the cashier logs back in.
            drop(state);
            state = start(folder.path());
            sign_in(&state, "owner", ADMIN_PASSWORD);
        }
        one_step(&state, &mut random, &mut expected, &card);
    }

    till::checkout_cancel(&state).expect("last cart cancelled");
    for held in till::checkout_held_list(&state).expect("held") {
        till::checkout_resume(&state, &held.cart_id).expect("resumed");
        till::checkout_cancel(&state).expect("cancelled");
    }
    let drawer = FLOAT + expected.cash_kept + expected.cash_moves;
    let closed = shift_commands::shift_close(
        &state,
        devices(),
        &[CountLine {
            denomination: Money(1),
            count: u32::try_from(drawer).expect("fits"),
        }],
        None,
    )
    .expect("closed");
    assert_eq!(closed.expected_cash, Money(drawer));
    assert_eq!(closed.difference, Some(Money(0)));
    check_saved_sales(&state, &expected);
    assert!(expected.returns > 0, "the day had returns");
    assert!(audit_count(&state, "sale.void") > 0, "the day had voids");
    let mismatches = state
        .db
        .read(|conn| Ok(inventory::verify(conn)?))
        .expect("verify");
    assert!(mismatches.is_empty(), "{mismatches:?}");
}

fn one_step(state: &AppState, random: &mut StdRng, expected: &mut Expected, card: &Id) {
    let cart = till::checkout_state(state)
        .expect("state")
        .cart
        .expect("cart");
    match random.random_range(0..100) {
        0..=49 => {
            let (code, _) = prices()[random.random_range(0..5)];
            let count = random.random_range(1..4);
            till::checkout_scan(state, &format!("{count}*{code}")).expect("scan");
        }
        50..=57 if !cart.lines.is_empty() => {
            let line = &cart.lines[random.random_range(0..cart.lines.len())];
            let qty = Qty(random.random_range(0..5) * 1000);
            till::checkout_set_qty(state, &line.line_id, qty).expect("qty");
        }
        58..=61 if !cart.lines.is_empty() => {
            till::checkout_hold(state, Some("later")).expect("held");
        }
        62..=65 => {
            let held = till::checkout_held_list(state).expect("held");
            if let Some(first) = held.first()
                && cart.lines.is_empty()
            {
                till::checkout_resume(state, &first.cart_id).expect("resumed");
            }
        }
        66..=68 => {
            till::checkout_cancel(state).expect("cancelled");
        }
        69..=71 => {
            let amount = random.random_range(1..50) * 100;
            let kind = if random.random_bool(0.5) {
                CashMoveKind::CashIn
            } else {
                CashMoveKind::CashOut
            };
            let input = CashMoveInput {
                kind,
                amount: Money(amount),
                reason: "float top-up".into(),
            };
            shift_commands::shift_cash_move(state, devices(), &input, None).expect("cash move");
            expected.cash_moves += if kind == CashMoveKind::CashIn {
                amount
            } else {
                -amount
            };
        }
        72..=75 if !expected.sold.is_empty() => give_back(state, random, expected),
        76..=77 if !expected.sold.is_empty() => void(state, random, expected),
        _ if !cart.lines.is_empty() => pay(state, random, expected, card),
        _ => {}
    }
}

/// Pays the cart in cash, by card, or split, and checks the change.
fn pay(state: &AppState, random: &mut StdRng, expected: &mut Expected, card: &Id) {
    let preview = till::checkout_payment_preview(state, &[]).expect("preview");
    let to_pay = preview.cash_to_pay.0;
    let payments: Vec<PaymentEntry> = match random.random_range(0..3) {
        0 => vec![cash(state, to_pay + random.random_range(0..3) * 1_000)],
        1 => vec![PaymentEntry {
            method_id: card.clone(),
            amount: preview.gross,
            reference: None,
        }],
        _ => {
            let part = preview.gross.0 / 3;
            let rest = till::checkout_payment_preview(
                state,
                &[PaymentEntry {
                    method_id: card.clone(),
                    amount: Money(part),
                    reference: None,
                }],
            )
            .expect("split preview")
            .cash_to_pay;
            vec![
                PaymentEntry {
                    method_id: card.clone(),
                    amount: Money(part),
                    reference: None,
                },
                cash(state, rest.0 + 500),
            ]
        }
    };
    let cash_given: i64 = payments
        .iter()
        .filter(|each| each.method_id != *card)
        .map(|each| each.amount.0)
        .sum();
    match till::checkout_complete(state, devices(), &payments) {
        Ok(done) => {
            expected.sales += 1;
            assert_eq!(done.invoice_no, format!("T1-{:06}", expected.sales));
            expected.cash_kept += cash_given - done.change.0;
            expected.sold.push((done.sale_id, done.invoice_no, true));
        }
        Err(error) => panic!("payment refused: {error:?} for {payments:?}"),
    }
}

/// Returns one piece of a random earlier sale, refunded the way it was paid (RET-01).
fn give_back(state: &AppState, random: &mut StdRng, expected: &mut Expected) {
    let pick = random.random_range(0..expected.sold.len());
    let found = return_commands::return_lookup(state, &expected.sold[pick].1).expect("found");
    let Some(line) = found.lines.iter().find(|line| line.returnable.0 >= 1_000) else {
        return;
    };
    let input = ReturnInput {
        sale_id: Some(found.sale_id.clone()),
        lines: vec![ReturnLineInput {
            sale_line_id: Some(line.sale_line_id.clone()),
            product_id: None,
            qty: Qty(1_000),
            restock: random.random_bool(0.8),
            unit_price: None,
        }],
        reason: Some("Changed mind".into()),
        refund_method_id: found.refund_method_id.clone(),
        reference: None,
    };
    let done = return_commands::return_complete(state, devices(), &input, None).expect("returned");
    expected.returns += 1;
    assert_eq!(done.return_no, format!("T1-R{:06}", expected.returns));
    expected.cash_kept -= done.cash_back.0;
    // A sale with a return can't be voided any more (RET-04).
    expected.sold[pick].2 = false;
}

/// Voids a random earlier sale that has no return and isn't void yet (RET-04).
fn void(state: &AppState, random: &mut StdRng, expected: &mut Expected) {
    let pick = random.random_range(0..expected.sold.len());
    let (sale_id, _, open) = &expected.sold[pick];
    if !open {
        return;
    }
    let done = return_commands::sale_void(state, devices(), sale_id, "Rang up twice", None)
        .expect("voided");
    expected.cash_kept -= done.cash_back.0;
    expected.sold[pick].2 = false;
}

fn check_saved_sales(state: &AppState, expected: &Expected) {
    let summary = state
        .db
        .read(|conn| Ok(sales::sales_summary(conn)?))
        .expect("summary");
    assert_eq!(
        summary.sale_count, expected.sales,
        "no completed sale was lost"
    );
    let rows: Vec<(String, i64, i64)> = state
        .db
        .read(|conn| {
            let mut statement = conn
                .prepare(
                    "SELECT s.invoice_no, s.total_minor,
                            (SELECT coalesce(sum(amount_minor), 0) FROM sales_payments p
                             WHERE p.doc_type = 'sale' AND p.doc_id = s.id)
                     FROM sales_sales s ORDER BY s.invoice_no",
                )
                .map_err(|error| AppError::internal(error.to_string()))?;
            let rows = statement
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                .map_err(|error| AppError::internal(error.to_string()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| AppError::internal(error.to_string()))?;
            Ok(rows)
        })
        .expect("sales");
    for (index, (invoice_no, total, paid)) in rows.iter().enumerate() {
        assert_eq!(invoice_no, &format!("T1-{:06}", index + 1), "gapless");
        assert_eq!(total, paid, "{invoice_no}: payments add up to the total");
    }
}
