//! Contract tests: the public API and views behave as the module page says.
//! Each test names the requirement it covers (docs/02 Requirements/Req - Shifts and Cash.md).

mod common;

use common::{NOW, movement, open_shift, taka, test_db, today, write};
use pos_core::{AppError, Id, Money};
use shifts::{DenominationCount, MovementKind, ShiftStatus, ShiftsError};

#[test]
fn migrations_belong_to_this_module() {
    assert_eq!(shifts::migrations().module, "shifts");
}

#[test]
fn migrations_are_numbered_in_order_and_run_on_an_empty_database() {
    let all = shifts::migrations().migrations;
    for (index, migration) in all.iter().enumerate() {
        assert_eq!(migration.version as usize, index + 1);
    }
    test_db();
}

// ---- SHF-01 Open a shift ----

#[test]
fn shf_01_open_starts_a_shift_with_the_float_as_expected_cash() {
    let mut conn = test_db();
    let (till, user) = (Id::new(), Id::new());
    let id = open_shift(&mut conn, &till, &user, taka(2_000));

    let shift = shifts::current(&conn, &till).unwrap().unwrap();
    assert_eq!(shift.id, id);
    assert_eq!(shift.till_id, till);
    assert_eq!(shift.user_id, user);
    assert_eq!(shift.status, ShiftStatus::Open);
    assert_eq!(shift.business_date, today());
    assert_eq!(shift.opened_at, NOW);
    assert_eq!(shift.opening_float, taka(2_000));
    assert_eq!(shifts::expected_cash(&conn, &id).unwrap(), taka(2_000));
}

#[test]
fn shf_01_open_with_a_denomination_count_keeps_the_count() {
    let mut conn = test_db();
    let (till, user) = (Id::new(), Id::new());
    let counts = [(taka(500), 3), (taka(100), 5)];
    let id = write(&mut conn, |tx| {
        shifts::open(tx, &till, &user, taka(2_000), &counts, today(), NOW).unwrap()
    });

    let summary = shifts::summary(&conn, &id).unwrap();
    assert_eq!(
        summary.open_counts,
        vec![
            DenominationCount {
                denomination: taka(100),
                count: 5
            },
            DenominationCount {
                denomination: taka(500),
                count: 3
            },
        ]
    );
}

#[test]
fn shf_01_a_count_that_does_not_add_up_to_the_float_is_refused() {
    let mut conn = test_db();
    let (till, user) = (Id::new(), Id::new());
    let error = write(&mut conn, |tx| {
        shifts::open(
            tx,
            &till,
            &user,
            taka(2_000),
            &[(taka(500), 3)],
            today(),
            NOW,
        )
        .unwrap_err()
    });
    assert_eq!(error, ShiftsError::Invalid("float_count_mismatch"));
}

#[test]
fn shf_01_a_negative_float_is_refused() {
    let mut conn = test_db();
    let (till, user) = (Id::new(), Id::new());
    let error = write(&mut conn, |tx| {
        shifts::open(tx, &till, &user, taka(-1), &[], today(), NOW).unwrap_err()
    });
    assert_eq!(error, ShiftsError::Invalid("float_negative"));
}

// ---- SHF-02 One open shift per till ----

#[test]
fn shf_02_a_second_open_shift_on_the_same_till_is_refused() {
    let mut conn = test_db();
    let till = Id::new();
    open_shift(&mut conn, &till, &Id::new(), taka(1_000));

    let error = write(&mut conn, |tx| {
        shifts::open(tx, &till, &Id::new(), taka(1_000), &[], today(), NOW).unwrap_err()
    });
    assert_eq!(error, ShiftsError::AlreadyOpen { till: till.clone() });
    assert!(matches!(
        AppError::from(error),
        AppError::Conflict { code, .. } if code == "shift_already_open"
    ));
}

#[test]
fn shf_02_the_database_index_refuses_a_second_open_shift() {
    let conn = test_db();
    let insert = "INSERT INTO shifts_shifts (id, till_id, user_id, business_date, status, opened_at, opening_float_minor)
                  VALUES (?1, 'TILL', 'USER', '2026-03-02', 'open', 0, 0)";
    conn.execute(insert, ["A"]).unwrap();
    assert!(conn.execute(insert, ["B"]).is_err());
}

#[test]
fn shf_02_other_tills_and_a_closed_till_can_open_a_shift() {
    let mut conn = test_db();
    let (till, user) = (Id::new(), Id::new());
    let first = open_shift(&mut conn, &till, &user, taka(1_000));
    open_shift(&mut conn, &Id::new(), &user, taka(1_000));

    write(&mut conn, |tx| {
        shifts::close(tx, &first, &[(taka(1_000), 1)], &user, false, None, NOW).unwrap()
    });
    assert_eq!(shifts::current(&conn, &till).unwrap(), None);

    let second = open_shift(&mut conn, &till, &user, taka(1_000));
    assert_eq!(shifts::current(&conn, &till).unwrap().unwrap().id, second);
}

// ---- SHF-03 Cash in and out ----

#[test]
fn shf_03_cash_in_cash_out_and_cash_drops_need_a_reason() {
    let mut conn = test_db();
    let user = Id::new();
    let shift = open_shift(&mut conn, &Id::new(), &user, taka(1_000));

    for (kind, amount) in [
        (MovementKind::CashIn, taka(100)),
        (MovementKind::CashOut, taka(-100)),
        (MovementKind::CashDrop, taka(-100)),
    ] {
        let error = write(&mut conn, |tx| {
            shifts::record(tx, &shift, movement(kind, "cash", amount, &user)).unwrap_err()
        });
        assert_eq!(error, ShiftsError::Invalid("reason_missing"));
    }
}

#[test]
fn shf_03_amounts_must_have_the_sign_of_their_kind() {
    let mut conn = test_db();
    let user = Id::new();
    let shift = open_shift(&mut conn, &Id::new(), &user, taka(1_000));

    for (kind, wrong_amount) in [
        (MovementKind::SalePayment, taka(-10)),
        (MovementKind::CustomerPayment, Money::zero()),
        (MovementKind::Refund, taka(10)),
        (MovementKind::SupplierPayment, taka(10)),
    ] {
        let error = write(&mut conn, |tx| {
            shifts::record(tx, &shift, movement(kind, "cash", wrong_amount, &user)).unwrap_err()
        });
        assert_eq!(error, ShiftsError::Invalid("amount_wrong_sign"), "{kind:?}");
    }
}

#[test]
fn float_and_void_reversal_rows_come_only_from_open_and_reverse_ref() {
    let mut conn = test_db();
    let user = Id::new();
    let shift = open_shift(&mut conn, &Id::new(), &user, taka(1_000));

    for kind in [MovementKind::Float, MovementKind::VoidReversal] {
        let error = write(&mut conn, |tx| {
            shifts::record(tx, &shift, movement(kind, "cash", taka(10), &user)).unwrap_err()
        });
        assert_eq!(error, ShiftsError::Invalid("movement_kind_not_allowed"));
    }
}

// ---- Ledger, views and errors ----

#[test]
fn movements_are_append_only() {
    let mut conn = test_db();
    let shift = open_shift(&mut conn, &Id::new(), &Id::new(), taka(1_000));
    assert!(
        conn.execute("UPDATE shifts_movements SET amount_minor = 0", [])
            .is_err()
    );
    assert!(conn.execute("DELETE FROM shifts_movements", []).is_err());
    assert_eq!(shifts::expected_cash(&conn, &shift).unwrap(), taka(1_000));
}

#[test]
fn view_shifts_v_movements_adds_business_date_and_till() {
    let mut conn = test_db();
    let (till, user) = (Id::new(), Id::new());
    let shift = open_shift(&mut conn, &till, &user, taka(1_000));
    let row: (String, String, String, String, i64, String) = conn
        .query_row(
            "SELECT shift_id, business_date, till_id, kind, amount_minor, method_kind
             FROM shifts_v_movements",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(
        row,
        (
            shift.to_string(),
            "2026-03-02".to_string(),
            till.to_string(),
            "float".to_string(),
            100_000,
            "cash".to_string()
        )
    );
}

#[test]
fn view_shifts_v_shifts_shows_every_shift_column() {
    let mut conn = test_db();
    let shift = open_shift(&mut conn, &Id::new(), &Id::new(), taka(1_000));
    let row: (String, String, i64, i64) = conn
        .query_row(
            "SELECT id, status, opening_float_minor, forced FROM shifts_v_shifts",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(row, (shift.to_string(), "open".to_string(), 100_000, 0));
    conn.query_row(
        "SELECT till_id, user_id, business_date, opened_at, closed_at, closed_by,
                expected_cash_minor, counted_cash_minor, difference_minor, close_note
         FROM shifts_v_shifts",
        [],
        |_| Ok(()),
    )
    .unwrap();
}

#[test]
fn shf_08_view_shifts_v_counts_publishes_the_opening_and_closing_counts() {
    let mut conn = test_db();
    let (till, user) = (Id::new(), Id::new());
    let shift = write(&mut conn, |tx| {
        shifts::open(
            tx,
            &till,
            &user,
            taka(600),
            &[(taka(500), 1), (taka(100), 1)],
            today(),
            NOW,
        )
        .unwrap()
    });
    write(&mut conn, |tx| {
        shifts::close(tx, &shift, &[(taka(100), 6)], &user, false, None, NOW).unwrap()
    });
    let mut statement = conn
        .prepare(
            "SELECT shift_id, stage, denomination_minor, count FROM shifts_v_counts
             ORDER BY stage DESC, denomination_minor",
        )
        .unwrap();
    let rows: Vec<(String, String, i64, i64)> = statement
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let id = shift.to_string();
    assert_eq!(
        rows,
        vec![
            (id.clone(), "open".to_string(), 10_000, 1),
            (id.clone(), "open".to_string(), 50_000, 1),
            (id, "close".to_string(), 10_000, 6),
        ]
    );
}

#[test]
fn an_unknown_shift_is_not_found() {
    let mut conn = test_db();
    let missing = Id::new();
    assert_eq!(
        shifts::expected_cash(&conn, &missing).unwrap_err(),
        ShiftsError::NotFound(missing.clone())
    );
    let error = write(&mut conn, |tx| {
        let sale = movement(MovementKind::SalePayment, "cash", taka(1), &Id::new());
        shifts::record(tx, &missing, sale).unwrap_err()
    });
    assert!(
        matches!(AppError::from(error), AppError::NotFound { entity, .. } if entity == "shift")
    );
}
