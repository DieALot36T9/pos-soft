//! Contract tests for the points and credit ledgers, voids and statements.

mod support;

use customers::{CreditKind, CustomersError, LedgerRef, PaymentMethod, PointsKind};
use pos_core::{BusinessDate, Id, Money, Timestamp};
use support::{COUNTRY, ctx, ctx_on, quick_customer, sale_ref, test_db};

#[test]
fn cus_04_points_rows_update_the_balance() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let rahim = quick_customer(&tx, "01711123456", "Rahim");
    let sale = sale_ref();

    let after_earn =
        customers::add_points(&tx, &rahim, 12, PointsKind::Earn, Some(&sale), None, &ctx());
    assert_eq!(after_earn.unwrap(), 12);
    let after_redeem = customers::add_points(
        &tx,
        &rahim,
        -5,
        PointsKind::Redeem,
        Some(&sale),
        None,
        &ctx(),
    );
    assert_eq!(after_redeem.unwrap(), 7);
    assert_eq!(customers::get(&tx, &rahim).unwrap().points, 7);
}

#[test]
fn cus_04_points_signs_and_balance_are_checked() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let rahim = quick_customer(&tx, "01711123456", "Rahim");
    let add = |delta, kind| customers::add_points(&tx, &rahim, delta, kind, None, None, &ctx());

    assert!(matches!(
        add(0, PointsKind::Earn),
        Err(CustomersError::AmountZero)
    ));
    assert!(matches!(
        add(-1, PointsKind::Earn),
        Err(CustomersError::AmountWrongSign { .. })
    ));
    assert!(matches!(
        add(1, PointsKind::Redeem),
        Err(CustomersError::AmountWrongSign { .. })
    ));
    assert!(matches!(
        add(-1, PointsKind::Redeem),
        Err(CustomersError::PointsNotEnough { balance: 0 })
    ));
}

#[test]
fn cus_04_admin_adjustments_need_a_reason() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let rahim = quick_customer(&tx, "01711123456", "Rahim");

    let no_reason = customers::add_points(&tx, &rahim, 50, PointsKind::Adjust, None, None, &ctx());
    assert!(matches!(no_reason, Err(CustomersError::ReasonMissing)));
    let blank = customers::add_points(&tx, &rahim, 50, PointsKind::Adjust, None, Some(" "), &ctx());
    assert!(matches!(blank, Err(CustomersError::ReasonMissing)));

    let reason = Some("Lost card");
    let ok = customers::add_points(&tx, &rahim, 50, PointsKind::Adjust, None, reason, &ctx());
    assert_eq!(ok.unwrap(), 50);
}

#[test]
fn cus_04_customers_who_opted_out_earn_no_points() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let mut details = customers::CustomerDetails::quick("01711123456", "Rahim");
    details.loyalty_opt_out = true;
    let rahim = customers::create(&tx, details, COUNTRY, Timestamp(1)).unwrap();

    let earn = customers::add_points(&tx, &rahim, 12, PointsKind::Earn, None, None, &ctx());
    assert!(matches!(earn, Err(CustomersError::LoyaltyOptedOut)));
}

#[test]
fn cus_04_cus_05_ledgers_refuse_unknown_customers() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let nobody = Id::new();
    let points = customers::add_points(&tx, &nobody, 1, PointsKind::Earn, None, None, &ctx());
    assert!(matches!(points, Err(CustomersError::CustomerNotFound(_))));
    let credit = customers::add_credit(
        &tx,
        &nobody,
        Money(100),
        CreditKind::CreditSale,
        None,
        None,
        &ctx(),
    );
    assert!(matches!(credit, Err(CustomersError::CustomerNotFound(_))));
}

#[test]
fn adr_010_ledgers_are_append_only() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let rahim = quick_customer(&tx, "01711123456", "Rahim");
    customers::add_points(&tx, &rahim, 5, PointsKind::Earn, None, None, &ctx()).unwrap();
    let kind = CreditKind::CreditSale;
    customers::add_credit(&tx, &rahim, Money(100), kind, None, None, &ctx()).unwrap();

    for sql in [
        "UPDATE customers_points_ledger SET points_delta = 1",
        "DELETE FROM customers_points_ledger",
        "UPDATE customers_credit_ledger SET amount_minor = 1",
        "DELETE FROM customers_credit_ledger",
    ] {
        assert!(tx.execute(sql, []).is_err(), "{sql} should be refused");
    }
}

#[test]
fn cus_05_credit_sales_and_payments_update_the_balance() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let rahim = quick_customer(&tx, "01711123456", "Rahim");

    let owed = customers::add_credit(
        &tx,
        &rahim,
        Money(80_000),
        CreditKind::CreditSale,
        None,
        Some(&sale_ref()),
        &ctx(),
    );
    assert_eq!(owed.unwrap(), Money(80_000));

    let bkash = PaymentMethod {
        method_kind: "bkash".to_string(),
        reference: Some("TX123".to_string()),
    };
    let after = customers::add_credit(
        &tx,
        &rahim,
        Money(-30_000),
        CreditKind::Payment,
        Some(&bkash),
        None,
        &ctx(),
    );
    assert_eq!(after.unwrap(), Money(50_000));
    assert_eq!(customers::get(&tx, &rahim).unwrap().credit, Money(50_000));
}

#[test]
fn cus_05_credit_signs_are_checked_and_payments_cannot_exceed_the_debt() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let rahim = quick_customer(&tx, "01711123456", "Rahim");
    let add =
        |amount, kind| customers::add_credit(&tx, &rahim, Money(amount), kind, None, None, &ctx());

    assert!(matches!(
        add(0, CreditKind::Adjust),
        Err(CustomersError::AmountZero)
    ));
    assert!(matches!(
        add(-5, CreditKind::CreditSale),
        Err(CustomersError::AmountWrongSign { .. })
    ));
    assert!(matches!(
        add(5, CreditKind::Payment),
        Err(CustomersError::AmountWrongSign { .. })
    ));
    assert!(matches!(
        add(5, CreditKind::Return),
        Err(CustomersError::AmountWrongSign { .. })
    ));
    assert!(matches!(
        add(5, CreditKind::StoreCredit),
        Err(CustomersError::AmountWrongSign { .. })
    ));
    assert!(matches!(
        add(-5, CreditKind::StoreCreditUsed),
        Err(CustomersError::AmountWrongSign { .. })
    ));

    add(1_000, CreditKind::CreditSale).unwrap();
    assert!(matches!(
        add(-1_001, CreditKind::Payment),
        Err(CustomersError::PaymentMoreThanDebt { owed: Money(1_000) })
    ));
}

#[test]
fn cus_06_store_credit_makes_the_balance_negative_and_can_be_spent() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let rahim = quick_customer(&tx, "01711123456", "Rahim");
    let add =
        |amount, kind| customers::add_credit(&tx, &rahim, Money(amount), kind, None, None, &ctx());

    assert_eq!(
        add(-50_000, CreditKind::StoreCredit).unwrap(),
        Money(-50_000)
    );
    assert_eq!(
        add(20_000, CreditKind::StoreCreditUsed).unwrap(),
        Money(-30_000)
    );
    assert!(matches!(
        add(30_001, CreditKind::StoreCreditUsed),
        Err(CustomersError::StoreCreditNotEnough {
            available: Money(30_000)
        })
    ));
}

#[test]
fn cus_06_store_credit_is_used_before_new_debt() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let rahim = quick_customer(&tx, "01711123456", "Rahim");
    let kind = CreditKind::StoreCredit;
    customers::add_credit(&tx, &rahim, Money(-30_000), kind, None, None, &ctx()).unwrap();

    // Pay 500.00 with "Customer credit": 300.00 of store credit, then 200.00 of new debt.
    let paid =
        customers::pay_with_credit(&tx, &rahim, Money(50_000), Some(&sale_ref()), &ctx()).unwrap();
    assert_eq!(paid.store_credit_used, Money(30_000));
    assert_eq!(paid.new_debt, Money(20_000));
    assert_eq!(customers::get(&tx, &rahim).unwrap().credit, Money(20_000));

    let kinds: Vec<String> = tx
        .prepare("SELECT kind FROM customers_credit_ledger ORDER BY rowid")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(kinds, ["store_credit", "store_credit_used", "credit_sale"]);
}

#[test]
fn cus_05_oldest_unpaid_debt_follows_first_in_first_out() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let rahim = quick_customer(&tx, "01711123456", "Rahim");
    let add = |amount, kind, at| {
        customers::add_credit(
            &tx,
            &rahim,
            Money(amount),
            kind,
            None,
            None,
            &ctx_on("2026-10-04", at),
        )
    };
    let oldest = || customers::get(&tx, &rahim).unwrap().oldest_unpaid_at;

    add(1_000, CreditKind::CreditSale, 100).unwrap();
    add(2_000, CreditKind::CreditSale, 200).unwrap();
    assert_eq!(oldest(), Some(Timestamp(100)));
    // Paying 1,000 clears the first debt, so the second one is now the oldest.
    add(-1_000, CreditKind::Payment, 300).unwrap();
    assert_eq!(oldest(), Some(Timestamp(200)));
    add(-2_000, CreditKind::Payment, 400).unwrap();
    assert_eq!(oldest(), None);
}

#[test]
fn cus_04_cus_05_void_reverses_every_row_of_the_document_once() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let rahim = quick_customer(&tx, "01711123456", "Rahim");
    let sale = sale_ref();
    let other_sale = sale_ref();
    customers::add_points(
        &tx,
        &rahim,
        100,
        PointsKind::Earn,
        Some(&other_sale),
        None,
        &ctx(),
    )
    .unwrap();
    customers::add_points(
        &tx,
        &rahim,
        -40,
        PointsKind::Redeem,
        Some(&sale),
        None,
        &ctx(),
    )
    .unwrap();
    customers::add_points(&tx, &rahim, 9, PointsKind::Earn, Some(&sale), None, &ctx()).unwrap();
    customers::pay_with_credit(&tx, &rahim, Money(70_000), Some(&sale), &ctx()).unwrap();

    customers::reverse_ref(&tx, "sale", &sale.ref_id, &ctx()).unwrap();

    let after = customers::get(&tx, &rahim).unwrap();
    assert_eq!(after.points, 100);
    assert_eq!(after.credit, Money::zero());
    assert_eq!(after.oldest_unpaid_at, None);

    let twice = customers::reverse_ref(&tx, "sale", &sale.ref_id, &ctx());
    assert!(matches!(twice, Err(CustomersError::AlreadyReversed { .. })));
}

#[test]
fn cus_05_void_of_a_document_with_no_customer_rows_does_nothing() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    customers::reverse_ref(&tx, "sale", &Id::new(), &ctx()).unwrap();
}

#[test]
fn cus_05_statement_shows_opening_lines_and_closing_balances() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let rahim = quick_customer(&tx, "01711123456", "Rahim");
    let add = |amount, kind, day, at| {
        customers::add_credit(
            &tx,
            &rahim,
            Money(amount),
            kind,
            None,
            None,
            &ctx_on(day, at),
        )
    };
    add(10_000, CreditKind::CreditSale, "2026-09-30", 1).unwrap();
    add(5_000, CreditKind::CreditSale, "2026-10-01", 2).unwrap();
    add(-3_000, CreditKind::Payment, "2026-10-02", 3).unwrap();
    add(1_000, CreditKind::CreditSale, "2026-10-05", 4).unwrap();

    let from = BusinessDate::parse("2026-10-01").unwrap();
    let to = BusinessDate::parse("2026-10-04").unwrap();
    let statement = customers::statement(&tx, &rahim, from, to).unwrap();

    assert_eq!(statement.opening, Money(10_000));
    assert_eq!(statement.lines.len(), 2);
    assert_eq!(statement.lines[0].kind, CreditKind::CreditSale);
    assert_eq!(statement.lines[0].balance_after, Money(15_000));
    assert_eq!(statement.lines[1].amount, Money(-3_000));
    assert_eq!(statement.lines[1].balance_after, Money(12_000));
    assert_eq!(statement.closing, Money(12_000));

    let backwards = customers::statement(&tx, &rahim, to, from);
    assert!(matches!(backwards, Err(CustomersError::DateRangeInvalid)));
}

#[test]
fn cus_05_statement_lines_keep_the_method_and_document() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let rahim = quick_customer(&tx, "01711123456", "Rahim");
    let receipt = LedgerRef {
        ref_type: "customer_payment".to_string(),
        ref_id: Id::new(),
    };
    let cash = PaymentMethod {
        method_kind: "cash".to_string(),
        reference: None,
    };
    let kind = CreditKind::CreditSale;
    customers::add_credit(&tx, &rahim, Money(500), kind, None, None, &ctx()).unwrap();
    customers::add_credit(
        &tx,
        &rahim,
        Money(-500),
        CreditKind::Payment,
        Some(&cash),
        Some(&receipt),
        &ctx(),
    )
    .unwrap();

    let day = BusinessDate::parse("2026-10-04").unwrap();
    let line = &customers::statement(&tx, &rahim, day, day).unwrap().lines[1];
    assert_eq!(line.method_kind.as_deref(), Some("cash"));
    assert_eq!(line.ref_type.as_deref(), Some("customer_payment"));
    assert_eq!(line.ref_id, Some(receipt.ref_id));
}
