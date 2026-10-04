//! Adjustments with reasons, and reversing a document's rows (void).
//! Requirements INV-02, INV-09 and RET-04 (a void reverses stock); ADR-010.

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

mod support;

use inventory::{BatchChoice, MoveKind, Movement, ReasonDirection};
use pos_core::{AppError, Id, Money, Qty};
use support::{batch, ctx, pieces, receive_into, test_db};

#[test]
fn inv_02_an_adjustment_needs_a_reason() {
    let mut connection = test_db();
    let transaction = connection.transaction().unwrap();
    let movement = Movement::new(&Id::new(), pieces(-1), MoveKind::Adjust, ctx());
    let error = inventory::record(&transaction, movement, true).unwrap_err();
    assert_eq!(AppError::from(error), AppError::validation("reason_needed"));
}

#[test]
fn inv_02_the_reason_must_fit_the_direction() {
    let mut connection = test_db();
    let transaction = connection.transaction().unwrap();
    // "damage" only takes stock out.
    let movement = Movement {
        reason_code: Some("damage".to_string()),
        ..Movement::new(&Id::new(), pieces(1), MoveKind::Adjust, ctx())
    };
    let error = inventory::record(&transaction, movement, true).unwrap_err();
    assert_eq!(
        AppError::from(error),
        AppError::validation("reason_wrong_direction")
    );
}

#[test]
fn inv_02_an_unknown_or_switched_off_reason_is_refused() {
    let mut connection = test_db();
    let transaction = connection.transaction().unwrap();
    inventory::add_reason(
        &transaction,
        "gift",
        "Gift to staff",
        inventory::ReasonDirection::Out,
    )
    .unwrap();
    inventory::set_reason_active(&transaction, "gift", false).unwrap();

    for code in ["no_such_reason", "gift"] {
        let movement = Movement {
            reason_code: Some(code.to_string()),
            ..Movement::new(&Id::new(), pieces(-1), MoveKind::Adjust, ctx())
        };
        let error = inventory::record(&transaction, movement, true).unwrap_err();
        assert_eq!(
            AppError::from(error),
            AppError::validation("reason_unknown")
        );
    }
}

#[test]
fn inv_02_reason_other_needs_a_note() {
    let mut connection = test_db();
    let product = Id::new();
    let transaction = connection.transaction().unwrap();
    let without_note = Movement {
        reason_code: Some("other".to_string()),
        ..Movement::new(&product, pieces(-1), MoveKind::Adjust, ctx())
    };
    let error = inventory::record(&transaction, without_note, true).unwrap_err();
    assert_eq!(AppError::from(error), AppError::validation("note_needed"));

    let with_note = Movement {
        reason_code: Some("other".to_string()),
        note: Some("Given to a charity".to_string()),
        ..Movement::new(&product, pieces(-1), MoveKind::Adjust, ctx())
    };
    inventory::record(&transaction, with_note, true).unwrap();
}

#[test]
fn ret_04_void_writes_the_exact_opposite_rows_on_the_same_batches() {
    let mut connection = test_db();
    let product = Id::new();
    let first = batch(&mut connection, &product, Some("2026-05-01"));
    let second = batch(&mut connection, &product, Some("2026-06-01"));
    receive_into(&mut connection, &product, &first, 2);
    receive_into(&mut connection, &product, &second, 2);
    let sale = Id::new();
    let transaction = connection.transaction().unwrap();
    let movement = Movement {
        batch: BatchChoice::Fefo,
        ref_: Some(("sale".to_string(), sale.clone())),
        unit_cost: Money(4_000),
        ..Movement::new(&product, pieces(-5), MoveKind::Sale, ctx())
    };
    let sold = inventory::record(&transaction, movement, true).unwrap();
    transaction.commit().unwrap();

    let transaction = connection.transaction().unwrap();
    let reversed =
        inventory::reverse(&transaction, "sale", &sale, MoveKind::SaleVoid, &ctx()).unwrap();
    transaction.commit().unwrap();

    let opposite: Vec<(Option<Id>, Qty)> = sold
        .iter()
        .map(|row| (row.batch_id.clone(), Qty(-row.qty_delta.0)))
        .collect();
    let got: Vec<(Option<Id>, Qty)> = reversed
        .iter()
        .map(|row| (row.batch_id.clone(), row.qty_delta))
        .collect();
    assert_eq!(got, opposite);
    assert_eq!(
        inventory::on_hand(&connection, &product).unwrap(),
        pieces(4)
    );
    let batches = inventory::batches_of(&connection, &product).unwrap();
    assert!(batches.iter().all(|row| row.on_hand == pieces(2)));
    // The reversal rows keep the cost and point at the same sale.
    let void_rows: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM inventory_ledger
             WHERE kind = 'sale_void' AND ref_type = 'sale' AND ref_id = ?1
               AND unit_cost_minor = 4000",
            [sale.as_str()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(void_rows, 3);
    assert!(inventory::verify(&connection).unwrap().is_empty());
}

#[test]
fn ret_04_voiding_twice_is_refused() {
    let mut connection = test_db();
    let product = Id::new();
    let sale = Id::new();
    let transaction = connection.transaction().unwrap();
    let movement = Movement {
        ref_: Some(("sale".to_string(), sale.clone())),
        ..Movement::new(&product, pieces(-1), MoveKind::Sale, ctx())
    };
    inventory::record(&transaction, movement, true).unwrap();
    inventory::reverse(&transaction, "sale", &sale, MoveKind::SaleVoid, &ctx()).unwrap();

    let error =
        inventory::reverse(&transaction, "sale", &sale, MoveKind::SaleVoid, &ctx()).unwrap_err();

    let AppError::Conflict { code, .. } = AppError::from(error) else {
        panic!("expected a conflict");
    };
    assert_eq!(code, "already_reversed");
}

#[test]
fn inv_02_built_in_reasons_are_seeded() {
    let connection = test_db();
    let reasons = inventory::list_reasons(&connection).unwrap();
    let mut codes: Vec<&str> = reasons.iter().map(|reason| reason.code.as_str()).collect();
    codes.sort_unstable();
    assert_eq!(
        codes,
        vec![
            "count_error",
            "damage",
            "expiry",
            "internal_use",
            "opening",
            "other",
            "theft"
        ]
    );
    assert!(
        reasons
            .iter()
            .all(|reason| reason.is_system && reason.active)
    );
}

#[test]
fn inv_02_shops_add_and_rename_their_own_reasons() {
    let mut connection = test_db();
    let transaction = connection.transaction().unwrap();
    inventory::add_reason(&transaction, "sample", "Free sample", ReasonDirection::Out).unwrap();
    inventory::rename_reason(&transaction, "sample", "Free tasting sample").unwrap();

    let reasons = inventory::list_reasons(&transaction).unwrap();
    let sample = reasons
        .iter()
        .find(|reason| reason.code == "sample")
        .unwrap();
    assert_eq!(sample.name, "Free tasting sample");
    assert_eq!(sample.direction, ReasonDirection::Out);
    assert!(!sample.is_system);

    let error =
        inventory::add_reason(&transaction, "sample", "Again", ReasonDirection::Out).unwrap_err();
    assert!(matches!(AppError::from(error), AppError::Conflict { .. }));
    let error =
        inventory::add_reason(&transaction, "Bad Code", "x", ReasonDirection::Out).unwrap_err();
    assert_eq!(
        AppError::from(error),
        AppError::validation("reason_code_invalid")
    );
    let error =
        inventory::add_reason(&transaction, "blank", "  ", ReasonDirection::Out).unwrap_err();
    assert_eq!(
        AppError::from(error),
        AppError::validation("reason_name_missing")
    );
}

#[test]
fn inv_02_built_in_reasons_cannot_be_switched_off() {
    let mut connection = test_db();
    let transaction = connection.transaction().unwrap();
    let error = inventory::set_reason_active(&transaction, "damage", false).unwrap_err();
    let AppError::Conflict { code, .. } = AppError::from(error) else {
        panic!("expected a conflict");
    };
    assert_eq!(code, "reason_is_built_in");

    inventory::add_reason(&transaction, "sample", "Free sample", ReasonDirection::Out).unwrap();
    inventory::set_reason_active(&transaction, "sample", false).unwrap();
    let reasons = inventory::list_reasons(&transaction).unwrap();
    assert!(
        !reasons
            .iter()
            .find(|reason| reason.code == "sample")
            .unwrap()
            .active
    );
}
