//! Random tests (proptest): the cached balances always equal the ledger sums.

mod support;

use customers::{CreditKind, PointsKind};
use pos_core::{Id, Money};
use proptest::prelude::*;
use rusqlite::Transaction;
use support::{ctx_on, quick_customer, sale_ref, test_db};

/// One random change to a customer's ledgers.
#[derive(Debug, Clone)]
enum Operation {
    Points(i64, PointsKind),
    Credit(i64, CreditKind),
    PayWithCredit(i64),
    VoidLastSale,
}

fn operation() -> impl Strategy<Value = Operation> {
    let points_kind = prop_oneof![
        Just(PointsKind::Earn),
        Just(PointsKind::Redeem),
        Just(PointsKind::Reverse),
        Just(PointsKind::Adjust),
    ];
    let credit_kind = prop_oneof![
        Just(CreditKind::CreditSale),
        Just(CreditKind::Payment),
        Just(CreditKind::Return),
        Just(CreditKind::StoreCredit),
        Just(CreditKind::StoreCreditUsed),
        Just(CreditKind::Adjust),
        Just(CreditKind::VoidReversal),
    ];
    prop_oneof![
        (-500i64..500, points_kind).prop_map(|(delta, kind)| Operation::Points(delta, kind)),
        (-50_000i64..50_000, credit_kind)
            .prop_map(|(amount, kind)| Operation::Credit(amount, kind)),
        (0i64..50_000).prop_map(Operation::PayWithCredit),
        Just(Operation::VoidLastSale),
    ]
}

/// Applies one operation. Refused operations are fine: they must change nothing.
fn apply(tx: &Transaction, customer: &Id, operation: &Operation, step: i64, last_sale: &mut Id) {
    let ctx = ctx_on("2026-10-04", step);
    let sale = sale_ref();
    let _ = match operation {
        Operation::Points(delta, kind) => {
            let reason = Some("test");
            customers::add_points(tx, customer, *delta, *kind, Some(&sale), reason, &ctx)
                .map(|_| ())
        }
        Operation::Credit(amount, kind) => {
            customers::add_credit(tx, customer, Money(*amount), *kind, None, Some(&sale), &ctx)
                .map(|_| ())
        }
        Operation::PayWithCredit(amount) => {
            customers::pay_with_credit(tx, customer, Money(*amount), Some(&sale), &ctx).map(|_| ())
        }
        Operation::VoidLastSale => customers::reverse_ref(tx, "sale", last_sale, &ctx),
    };
    *last_sale = sale.ref_id;
}

fn ledger_sums(tx: &Transaction, customer: &Id) -> (i64, i64) {
    let points: i64 = tx
        .query_row(
            "SELECT COALESCE(SUM(points_delta), 0) FROM customers_points_ledger WHERE customer_id = ?1",
            [customer.as_str()],
            |row| row.get(0),
        )
        .expect("sum a ledger");
    let credit: i64 = tx
        .query_row(
            "SELECT COALESCE(SUM(amount_minor), 0) FROM customers_credit_ledger WHERE customer_id = ?1",
            [customer.as_str()],
            |row| row.get(0),
        )
        .expect("sum a ledger");
    (points, credit)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn cus_04_cus_05_balances_equal_ledger_sums_after_random_operations(
        operations in prop::collection::vec((0usize..2, operation()), 1..40)
    ) {
        let mut db = test_db();
        let tx = db.transaction().unwrap();
        let people = [
            quick_customer(&tx, "01711123456", "Rahim"),
            quick_customer(&tx, "01811123456", "Karim"),
        ];
        let mut last_sale = Id::new();

        for (step, (who, operation)) in operations.iter().enumerate() {
            let step = i64::try_from(step).unwrap();
            apply(&tx, &people[*who], operation, step, &mut last_sale);

            for customer in &people {
                let found = customers::get(&tx, customer).unwrap();
                let (points, credit) = ledger_sums(&tx, customer);
                prop_assert_eq!(found.points, points);
                prop_assert_eq!(found.credit, Money(credit));
                // Something is owed exactly when there is an oldest unpaid debt.
                prop_assert_eq!(found.oldest_unpaid_at.is_some(), credit > 0);
            }
        }
    }
}
