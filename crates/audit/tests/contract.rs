//! Contract tests: the public API and the `audit_v_events` view behave as
//! `docs/05 Modules/Module - audit.md` says. Requirement: USR-08 (audit log).

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

use audit::{AuditCtx, AuditEvent, AuditFilter, AuditPage, AuditRow, migrations, query, record};
use pos_core::{AppError, BusinessDate, Id, Timestamp};
use rusqlite::Connection;
use serde_json::json;

/// An empty in-memory database with only this module's migrations (rule R6).
fn test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    for migration in migrations().migrations {
        conn.execute_batch(migration.sql).unwrap();
    }
    conn
}

fn date(text: &str) -> BusinessDate {
    BusinessDate::parse(text).unwrap()
}

fn ctx(at: i64, business_date: &str, till_id: &Id) -> AuditCtx {
    AuditCtx {
        at: Timestamp(at),
        business_date: date(business_date),
        till_id: till_id.clone(),
    }
}

fn event(action: &'static str, user_id: Option<&Id>) -> AuditEvent {
    AuditEvent {
        action,
        entity_type: None,
        entity_id: None,
        details: json!({}),
        user_id: user_id.cloned(),
        approved_by: None,
    }
}

/// Records one event in its own transaction and returns its id.
fn record_one(conn: &mut Connection, ev: AuditEvent, context: &AuditCtx) -> Id {
    let tx = conn.transaction().unwrap();
    let id = record(&tx, ev, context).unwrap();
    tx.commit().unwrap();
    id
}

fn all() -> AuditPage {
    AuditPage {
        limit: 100,
        offset: 0,
    }
}

fn actions(rows: &[AuditRow]) -> Vec<&str> {
    rows.iter().map(|row| row.action.as_str()).collect()
}

/// A small log: three users' actions over three business days.
struct Sample {
    conn: Connection,
    cashier: Id,
    manager: Id,
}

fn sample() -> Sample {
    let mut conn = test_db();
    let till = Id::new();
    let cashier = Id::new();
    let manager = Id::new();
    record_one(
        &mut conn,
        event("user.login", Some(&cashier)),
        &ctx(1_000, "2026-03-01", &till),
    );
    record_one(
        &mut conn,
        event("sale.void", Some(&cashier)),
        &ctx(2_000, "2026-03-01", &till),
    );
    record_one(
        &mut conn,
        event("price.change", Some(&manager)),
        &ctx(3_000, "2026-03-02", &till),
    );
    record_one(
        &mut conn,
        event("sale.void", Some(&manager)),
        &ctx(4_000, "2026-03-03", &till),
    );
    record_one(
        &mut conn,
        event("backup.made", None),
        &ctx(5_000, "2026-03-03", &till),
    );
    Sample {
        conn,
        cashier,
        manager,
    }
}

#[test]
fn migrations_belong_to_this_module() {
    let list = migrations();
    assert_eq!(list.module, "audit");
    assert_eq!(list.migrations[0].version, 1);
}

#[test]
fn usr_08_record_then_query_returns_every_field() {
    let mut conn = test_db();
    let till = Id::new();
    let cashier = Id::new();
    let admin = Id::new();
    let sale = Id::new();
    let ev = AuditEvent {
        action: "sale.void",
        entity_type: Some("sale"),
        entity_id: Some(sale.clone()),
        details: json!({ "invoice_no": "T1-000123", "reason": "wrong item" }),
        user_id: Some(cashier.clone()),
        approved_by: Some(admin.clone()),
    };

    let id = record_one(&mut conn, ev, &ctx(1_700_000_000_000, "2026-03-01", &till));

    let rows = query(&conn, &AuditFilter::default(), all()).unwrap();
    assert_eq!(
        rows,
        vec![AuditRow {
            id,
            at: Timestamp(1_700_000_000_000),
            business_date: date("2026-03-01"),
            till_id: till,
            user_id: Some(cashier),
            approved_by: Some(admin),
            action: "sale.void".to_string(),
            entity_type: Some("sale".to_string()),
            entity_id: Some(sale),
            details: json!({ "invoice_no": "T1-000123", "reason": "wrong item" }),
        }]
    );
}

#[test]
fn usr_08_system_events_have_no_user() {
    let sample = sample();
    let rows = query(&sample.conn, &AuditFilter::default(), all()).unwrap();
    let backup = rows.iter().find(|row| row.action == "backup.made").unwrap();
    assert_eq!(backup.user_id, None);
    assert_eq!(backup.details, json!({}));
}

#[test]
fn usr_08_nobody_can_change_an_event() {
    let sample = sample();
    let result = sample
        .conn
        .execute("UPDATE audit_events SET action = 'nothing'", []);
    let message = result.unwrap_err().to_string();
    assert!(message.contains("audit is append-only"), "{message}");
}

#[test]
fn usr_08_nobody_can_delete_an_event() {
    let sample = sample();
    let result = sample.conn.execute("DELETE FROM audit_events", []);
    let message = result.unwrap_err().to_string();
    assert!(message.contains("audit is append-only"), "{message}");

    let rows = query(&sample.conn, &AuditFilter::default(), all()).unwrap();
    assert_eq!(rows.len(), 5);
}

#[test]
fn usr_08_newest_events_come_first() {
    let sample = sample();
    let rows = query(&sample.conn, &AuditFilter::default(), all()).unwrap();
    assert_eq!(
        actions(&rows),
        vec![
            "backup.made",
            "sale.void",
            "price.change",
            "sale.void",
            "user.login"
        ]
    );
}

#[test]
fn usr_08_filter_by_business_date_range() {
    let sample = sample();
    let filter = AuditFilter {
        from: Some(date("2026-03-02")),
        to: Some(date("2026-03-03")),
        ..AuditFilter::default()
    };
    let rows = query(&sample.conn, &filter, all()).unwrap();
    assert_eq!(
        actions(&rows),
        vec!["backup.made", "sale.void", "price.change"]
    );

    // Both ends are included, so one day is from = to.
    let one_day = AuditFilter {
        from: Some(date("2026-03-01")),
        to: Some(date("2026-03-01")),
        ..AuditFilter::default()
    };
    let rows = query(&sample.conn, &one_day, all()).unwrap();
    assert_eq!(actions(&rows), vec!["sale.void", "user.login"]);
}

#[test]
fn usr_08_filter_by_user() {
    let sample = sample();
    let filter = AuditFilter {
        user_id: Some(sample.manager.clone()),
        ..AuditFilter::default()
    };
    let rows = query(&sample.conn, &filter, all()).unwrap();
    assert_eq!(actions(&rows), vec!["sale.void", "price.change"]);
    assert!(
        rows.iter()
            .all(|row| row.user_id.as_ref() == Some(&sample.manager))
    );
}

#[test]
fn usr_08_filter_by_action() {
    let sample = sample();
    let filter = AuditFilter {
        action: Some("sale.void".to_string()),
        ..AuditFilter::default()
    };
    let rows = query(&sample.conn, &filter, all()).unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.action == "sale.void"));
}

#[test]
fn usr_08_filters_combine() {
    let sample = sample();
    let filter = AuditFilter {
        from: Some(date("2026-03-01")),
        to: Some(date("2026-03-01")),
        user_id: Some(sample.cashier.clone()),
        action: Some("sale.void".to_string()),
    };
    let rows = query(&sample.conn, &filter, all()).unwrap();
    assert_eq!(actions(&rows), vec!["sale.void"]);
    assert_eq!(rows[0].at, Timestamp(2_000));
}

#[test]
fn usr_08_paging_walks_through_every_event_once() {
    let sample = sample();
    let first = query(
        &sample.conn,
        &AuditFilter::default(),
        AuditPage {
            limit: 2,
            offset: 0,
        },
    )
    .unwrap();
    let second = query(
        &sample.conn,
        &AuditFilter::default(),
        AuditPage {
            limit: 2,
            offset: 2,
        },
    )
    .unwrap();
    let third = query(
        &sample.conn,
        &AuditFilter::default(),
        AuditPage {
            limit: 2,
            offset: 4,
        },
    )
    .unwrap();
    assert_eq!(actions(&first), vec!["backup.made", "sale.void"]);
    assert_eq!(actions(&second), vec!["price.change", "sale.void"]);
    assert_eq!(actions(&third), vec!["user.login"]);
}

#[test]
fn usr_08_events_at_the_same_moment_keep_a_fixed_order() {
    let mut conn = test_db();
    let till = Id::new();
    let first = record_one(
        &mut conn,
        event("drawer.no_sale", None),
        &ctx(1_000, "2026-03-01", &till),
    );
    let second = record_one(
        &mut conn,
        event("drawer.no_sale", None),
        &ctx(1_000, "2026-03-01", &till),
    );
    let rows = query(&conn, &AuditFilter::default(), all()).unwrap();
    // Same time: the one recorded last comes first, as with every other event.
    let ids: Vec<Id> = rows.into_iter().map(|row| row.id).collect();
    assert_eq!(ids, vec![second, first]);
}

#[test]
fn usr_08_page_size_must_be_between_1_and_the_maximum() {
    let conn = test_db();
    for limit in [0, AuditPage::MAX_LIMIT + 1] {
        let result = query(
            &conn,
            &AuditFilter::default(),
            AuditPage { limit, offset: 0 },
        );
        assert_eq!(result, Err(AppError::validation("page_limit_invalid")));
    }
    let largest = AuditPage {
        limit: AuditPage::MAX_LIMIT,
        offset: 0,
    };
    assert!(query(&conn, &AuditFilter::default(), largest).is_ok());
}

#[test]
fn usr_08_event_is_lost_when_the_callers_transaction_is_rolled_back() {
    let mut conn = test_db();
    let till = Id::new();
    let tx = conn.transaction().unwrap();
    record(
        &tx,
        event("sale.void", None),
        &ctx(1_000, "2026-03-01", &till),
    )
    .unwrap();
    drop(tx); // dropping without commit rolls back

    let rows = query(&conn, &AuditFilter::default(), all()).unwrap();
    assert!(rows.is_empty());
}

#[test]
fn usr_08_badly_named_actions_are_refused() {
    let mut conn = test_db();
    let till = Id::new();
    let tx = conn.transaction().unwrap();
    for action in ["", "Sale.Void", "sale void", "sale.", ".void"] {
        let result = record(&tx, event(action, None), &ctx(1, "2026-03-01", &till));
        assert!(
            matches!(result, Err(AppError::Internal { .. })),
            "{action:?} was accepted"
        );
    }
}

#[test]
fn usr_08_details_must_be_a_json_object() {
    let mut conn = test_db();
    let till = Id::new();
    let tx = conn.transaction().unwrap();
    let mut ev = event("sale.void", None);
    ev.details = json!(["not", "an", "object"]);
    let result = record(&tx, ev, &ctx(1, "2026-03-01", &till));
    assert!(matches!(result, Err(AppError::Internal { .. })));
}

#[test]
fn usr_08_details_never_hold_secrets() {
    let mut conn = test_db();
    let till = Id::new();
    let tx = conn.transaction().unwrap();
    let mut ev = event("user.pin_change", None);
    // A secret hidden one level down is still found.
    ev.details = json!({ "user": { "name": "Rina", "pin": "1234" } });
    let result = record(&tx, ev, &ctx(1, "2026-03-01", &till));
    match result {
        Err(AppError::Internal { detail, .. }) => {
            assert!(detail.contains("pin"), "{detail}");
            assert!(!detail.contains("1234"), "{detail}");
        }
        other => panic!("expected an internal error, got {other:?}"),
    }
}

#[test]
fn usr_08_view_publishes_every_column() {
    let sample = sample();
    let statement = sample.conn.prepare("SELECT * FROM audit_v_events").unwrap();
    let columns: Vec<String> = statement
        .column_names()
        .into_iter()
        .map(str::to_string)
        .collect();
    assert_eq!(
        columns,
        vec![
            "id",
            "at",
            "business_date",
            "till_id",
            "user_id",
            "approved_by",
            "action",
            "entity_type",
            "entity_id",
            "details_json",
        ]
    );
    let count: i64 = sample
        .conn
        .query_row("SELECT count(*) FROM audit_v_events", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 5);
}

#[test]
fn usr_08_view_stores_dates_as_text_and_details_as_json_text() {
    let sample = sample();
    let (business_date, details_json): (String, String) = sample
        .conn
        .query_row(
            "SELECT business_date, details_json FROM audit_v_events WHERE action = 'price.change'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(business_date, "2026-03-02");
    assert_eq!(details_json, "{}");
}

#[test]
fn query_rows_are_sent_to_the_ui_in_snake_case() {
    let sample = sample();
    let rows = query(
        &sample.conn,
        &AuditFilter {
            action: Some("price.change".to_string()),
            ..AuditFilter::default()
        },
        all(),
    )
    .unwrap();
    let sent = serde_json::to_value(&rows[0]).unwrap();
    assert_eq!(sent["business_date"], "2026-03-02");
    assert_eq!(sent["at"], 3_000);
    assert_eq!(sent["details"], json!({}));
    assert_eq!(sent["user_id"], sample.manager.as_str());
}
