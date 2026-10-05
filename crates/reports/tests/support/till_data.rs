//! Shifts, cash movements and audit events of the fixture shop.

use rusqlite::{Connection, named_params};

use super::{id, local_ms};

fn add_shift(
    conn: &Connection,
    tag: &str,
    till: &str,
    user: &str,
    date: &str,
    opened: i64,
    float: i64,
) {
    conn.execute(
        "INSERT INTO shifts_shifts (id, till_id, user_id, business_date, status, opened_at, opening_float_minor)
         VALUES (:id, :till, :user, :date, 'open', :opened, :float)",
        named_params! {
            ":id": id(tag).as_str(), ":till": id(till).as_str(), ":user": id(user).as_str(),
            ":date": date, ":opened": opened, ":float": float,
        },
    )
    .unwrap();
    add_movement(conn, tag, "float", "cash", float, opened);
}

fn add_movement(
    conn: &Connection,
    shift: &str,
    kind: &str,
    method_kind: &str,
    amount: i64,
    at: i64,
) {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM shifts_movements", [], |row| {
            row.get(0)
        })
        .unwrap();
    conn.execute(
        "INSERT INTO shifts_movements (id, shift_id, kind, method_kind, amount_minor, at, user_id)
         VALUES (:id, :shift, :kind, :method_kind, :amount, :at, :user)",
        named_params! {
            ":id": id(&format!("MV{count}")).as_str(), ":shift": id(shift).as_str(), ":kind": kind,
            ":method_kind": method_kind, ":amount": amount, ":at": at, ":user": id("R").as_str(),
        },
    )
    .unwrap();
}

/// Shift SH1 (T1, Rina) is closed: expected 935.00, counted 930.00, short by 5.00.
/// SH3 (T2, Karim) is still open. SH2 is Rina's shift on 03-06.
pub fn add_shifts(conn: &Connection) {
    add_shift(
        conn,
        "SH1",
        "T1",
        "R",
        "2026-03-05",
        local_ms(0, 9, 0),
        100_000,
    );
    add_movement(
        conn,
        "SH1",
        "sale_payment",
        "cash",
        25_000,
        local_ms(0, 10, 15),
    );
    add_movement(
        conn,
        "SH1",
        "sale_payment",
        "card",
        12_500,
        local_ms(0, 11, 30),
    );
    add_movement(
        conn,
        "SH1",
        "sale_payment",
        "cash",
        13_500,
        local_ms(0, 12, 0),
    );
    add_movement(
        conn,
        "SH1",
        "sale_payment",
        "cash",
        4_000,
        local_ms(0, 13, 0),
    );
    add_movement(
        conn,
        "SH1",
        "void_reversal",
        "cash",
        -4_000,
        local_ms(0, 13, 10),
    );
    add_movement(conn, "SH1", "refund", "cash", -10_000, local_ms(0, 15, 0));
    add_movement(conn, "SH1", "cash_in", "cash", 20_000, local_ms(0, 15, 30));
    add_movement(conn, "SH1", "cash_out", "cash", -5_000, local_ms(0, 16, 30));
    add_movement(
        conn,
        "SH1",
        "cash_drop",
        "cash",
        -50_000,
        local_ms(0, 17, 0),
    );
    conn.execute(
        "UPDATE shifts_shifts SET status = 'closed', closed_at = :closed, closed_by = :user,
           expected_cash_minor = 93500, counted_cash_minor = 93000, difference_minor = -500
         WHERE id = :id",
        named_params! {
            ":closed": local_ms(0, 18, 0), ":user": id("R").as_str(), ":id": id("SH1").as_str(),
        },
    )
    .unwrap();
    add_shift(
        conn,
        "SH2",
        "T1",
        "R",
        "2026-03-06",
        local_ms(1, 9, 0),
        100_000,
    );
    add_shift(
        conn,
        "SH3",
        "T2",
        "K",
        "2026-03-05",
        local_ms(0, 15, 45),
        50_000,
    );
    add_movement(
        conn,
        "SH3",
        "sale_payment",
        "cash",
        4_000,
        local_ms(0, 16, 0),
    );
    add_counts(conn);
}

pub fn add_event(
    conn: &Connection,
    action: &str,
    till: &str,
    user: &str,
    approved_by: Option<&str>,
    at: i64,
) {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM audit_events", [], |row| row.get(0))
        .unwrap();
    conn.execute(
        "INSERT INTO audit_events (id, at, business_date, till_id, user_id, approved_by, action)
         VALUES (:id, :at, '2026-03-05', :till, :user, :approved_by, :action)",
        named_params! {
            ":id": id(&format!("EV{count}")).as_str(), ":at": at, ":till": id(till).as_str(),
            ":user": id(user).as_str(), ":approved_by": approved_by.map(|tag| id(tag).to_string()),
            ":action": action,
        },
    )
    .unwrap();
}

pub fn add_audit(conn: &Connection) {
    add_event(
        conn,
        "line.price_override",
        "T1",
        "R",
        Some("A"),
        local_ms(0, 11, 58),
    );
    add_event(conn, "cart.cancel", "T1", "R", None, local_ms(0, 12, 30));
    add_event(conn, "sale.void", "T1", "R", Some("A"), local_ms(0, 13, 10));
    add_event(conn, "drawer.no_sale", "T1", "R", None, local_ms(0, 14, 0));
    add_event(conn, "cart.cancel", "T2", "K", None, local_ms(0, 16, 10));
}

/// SH1 opened with 2 × 500.00 and was counted at 930.00: 1 × 500, 4 × 100, 3 × 10.
fn add_counts(conn: &Connection) {
    let counts = [
        ("open", 50_000, 2),
        ("close", 50_000, 1),
        ("close", 10_000, 4),
        ("close", 1_000, 3),
    ];
    for (stage, denomination, count) in counts {
        conn.execute(
            "INSERT INTO shifts_counts (shift_id, stage, denomination_minor, count)
             VALUES (:shift, :stage, :denomination, :count)",
            named_params! {
                ":shift": id("SH1").as_str(), ":stage": stage,
                ":denomination": denomination, ":count": count,
            },
        )
        .unwrap();
    }
}
