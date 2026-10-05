//! Randomized test: any sequence of stock changes keeps every cached level and
//! batch equal to the sum of its ledger rows. Requirement INV-10 (ADR-010).

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

mod support;

use std::collections::HashMap;

use inventory::{BatchChoice, CountEntry, CountScope, MoveKind, Movement, NewCount};
use pos_core::{Id, Money, Qty, Timestamp};
use proptest::prelude::*;
use rusqlite::Connection;
use support::{ctx, date, test_db, user};

/// One random step. Product and batch numbers pick from small lists, so steps
/// often touch the same product and batch.
#[derive(Debug, Clone)]
enum Step {
    Receive {
        product: usize,
        batch: Option<usize>,
        milli: i64,
    },
    SellFefo {
        product: usize,
        milli: i64,
        allow_negative: bool,
    },
    SellNoBatch {
        product: usize,
        milli: i64,
        allow_negative: bool,
    },
    AdjustBatch {
        product: usize,
        batch: usize,
        milli: i64,
        allow_negative: bool,
    },
    VoidSale {
        sale: usize,
    },
    Count {
        product: usize,
        counted_milli: i64,
        full: bool,
    },
}

fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        (0..3usize, prop::option::of(0..3usize), 1..20_000i64).prop_map(
            |(product, batch, milli)| Step::Receive {
                product,
                batch,
                milli
            }
        ),
        (0..3usize, 1..20_000i64, any::<bool>()).prop_map(|(product, milli, allow_negative)| {
            Step::SellFefo {
                product,
                milli,
                allow_negative,
            }
        }),
        (0..3usize, 1..20_000i64, any::<bool>()).prop_map(|(product, milli, allow_negative)| {
            Step::SellNoBatch {
                product,
                milli,
                allow_negative,
            }
        }),
        (0..3usize, 0..3usize, -10_000..10_000i64, any::<bool>())
            .prop_filter("not zero", |(_, _, milli, _)| *milli != 0)
            .prop_map(
                |(product, batch, milli, allow_negative)| Step::AdjustBatch {
                    product,
                    batch,
                    milli,
                    allow_negative,
                }
            ),
        (0..10usize).prop_map(|sale| Step::VoidSale { sale }),
        (0..3usize, 0..20_000i64, any::<bool>()).prop_map(|(product, counted_milli, full)| {
            Step::Count {
                product,
                counted_milli,
                full,
            }
        }),
    ]
}

/// The shop in the test: three products with three batches each, and the sales made.
struct Shop {
    connection: Connection,
    products: Vec<Id>,
    batches: Vec<Vec<Id>>,
    sales: Vec<Id>,
}

impl Shop {
    fn new() -> Shop {
        let mut connection = test_db();
        let products: Vec<Id> = (0..3).map(|_| Id::new()).collect();
        let expiries = [Some("2026-05-01"), Some("2026-04-01"), None];
        let transaction = connection.transaction().unwrap();
        let mut batches = Vec::new();
        for product in &products {
            let mut of_product = Vec::new();
            for expiry in expiries {
                let id = inventory::create_batch(
                    &transaction,
                    product,
                    None,
                    expiry.map(date),
                    Money(100),
                    Timestamp(1),
                )
                .unwrap();
                of_product.push(id);
            }
            batches.push(of_product);
        }
        transaction.commit().unwrap();
        Shop {
            connection,
            products,
            batches,
            sales: Vec::new(),
        }
    }

    /// Runs one step in its own transaction. A refused step saves nothing, as in the app.
    fn run(&mut self, step: &Step) {
        let transaction = self.connection.transaction().unwrap();
        let outcome = match step {
            Step::Receive {
                product,
                batch,
                milli,
            } => {
                let choice = match batch {
                    Some(index) => BatchChoice::Specific(self.batches[*product][*index].clone()),
                    None => BatchChoice::None,
                };
                let movement = Movement {
                    batch: choice,
                    ..Movement::new(
                        &self.products[*product],
                        Qty(*milli),
                        MoveKind::Receive,
                        ctx(),
                    )
                };
                inventory::record(&transaction, movement, true).map(|_| ())
            }
            Step::SellFefo {
                product,
                milli,
                allow_negative,
            } => {
                let sale = Id::new();
                let movement = Movement {
                    batch: BatchChoice::Fefo,
                    ref_: Some(("sale".to_string(), sale.clone())),
                    ..Movement::new(&self.products[*product], Qty(-milli), MoveKind::Sale, ctx())
                };
                let result = inventory::record(&transaction, movement, *allow_negative);
                if result.is_ok() {
                    self.sales.push(sale);
                }
                result.map(|_| ())
            }
            Step::SellNoBatch {
                product,
                milli,
                allow_negative,
            } => {
                let movement =
                    Movement::new(&self.products[*product], Qty(-milli), MoveKind::Sale, ctx());
                inventory::record(&transaction, movement, *allow_negative).map(|_| ())
            }
            Step::AdjustBatch {
                product,
                batch,
                milli,
                allow_negative,
            } => {
                let movement = Movement {
                    batch: BatchChoice::Specific(self.batches[*product][*batch].clone()),
                    reason_code: Some("count_error".to_string()),
                    ..Movement::new(
                        &self.products[*product],
                        Qty(*milli),
                        MoveKind::Adjust,
                        ctx(),
                    )
                };
                inventory::record(&transaction, movement, *allow_negative).map(|_| ())
            }
            Step::VoidSale { sale } => match self.sales.get(*sale) {
                Some(id) => {
                    inventory::reverse(&transaction, "sale", id, MoveKind::SaleVoid, &ctx())
                        .map(|_| ())
                }
                None => Ok(()),
            },
            Step::Count {
                product,
                counted_milli,
                full,
            } => count(
                &transaction,
                &self.products[*product],
                *counted_milli,
                *full,
            ),
        };
        if outcome.is_ok() {
            transaction.commit().unwrap();
        }
    }
}

/// Creates a count, counts one product and posts it.
fn count(
    transaction: &rusqlite::Transaction,
    product: &Id,
    counted_milli: i64,
    full: bool,
) -> Result<(), inventory::InventoryError> {
    let scope = if full {
        CountScope::Full
    } else {
        CountScope::Partial
    };
    let new_count = NewCount {
        name: "Random".to_string(),
        scope,
        scope_details: serde_json::json!({}),
    };
    let created = inventory::create_count(transaction, &new_count, Timestamp(2), &user())?;
    let entry = CountEntry {
        product_id: product.clone(),
        batch_id: None,
        qty: Qty(counted_milli),
    };
    inventory::record_count_line(transaction, &created.id, &entry, Timestamp(3), &user())?;
    inventory::post_count(transaction, &created.id, &HashMap::new(), &ctx())?;
    Ok(())
}

/// The sum of a product's ledger rows, read straight from the table.
fn ledger_sum(connection: &Connection, product: &Id) -> i64 {
    connection
        .query_row(
            "SELECT COALESCE(SUM(qty_delta_milli), 0) FROM inventory_ledger WHERE product_id = ?1",
            [product.as_str()],
            |row| row.get(0),
        )
        .unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn inv_10_levels_and_batches_always_equal_their_ledger_sums(
        steps in prop::collection::vec(step(), 1..40),
    ) {
        let mut shop = Shop::new();
        for step in &steps {
            shop.run(step);
        }

        prop_assert!(inventory::verify(&shop.connection).unwrap().is_empty());
        for product in &shop.products {
            let on_hand = inventory::on_hand(&shop.connection, product).unwrap();
            prop_assert_eq!(on_hand.0, ledger_sum(&shop.connection, product));
        }
    }
}
