//! M9 property tests (Roadmap: "Tax and money reverse exactly"): random sales with
//! random quantities, prices and a sale discount, returned in random pieces until
//! nothing is left, or voided. The refunds add up to the sale exactly (net, tax, gross
//! and cost), and the stock comes back to where it started.

mod common;

use app::commands::checkout as till;
use app::commands::returns as return_commands;
use app::state::AppState;
use app::workflows::returns::{ReturnInput, ReturnLineInput};
use common::*;
use pos_core::{Id, Money, Qty};
use pricing::ManualDiscount;
use proptest::prelude::*;

/// A sale to make: (price in minor units, quantity in whole pieces) per product,
/// and a discount on the whole sale.
#[derive(Debug, Clone)]
struct SalePlan {
    lines: Vec<(i64, i64)>,
    discount_minor: i64,
}

fn sale_plan() -> impl Strategy<Value = SalePlan> {
    (
        prop::collection::vec((1i64..50_000, 1i64..7), 1..4),
        0i64..3_000,
    )
        .prop_map(|(lines, discount_minor)| SalePlan {
            lines,
            discount_minor,
        })
}

/// A shop with one product per planned line, each with 100 in stock, the admin signed
/// in with the shift open, and the sale made. Gives the products and the sale's id.
fn sold(plan: &SalePlan) -> (tempfile::TempDir, AppState, Vec<Id>, Id) {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    open_shift(&state, 0);
    let mut products = Vec::new();
    for (index, (price, qty)) in plan.lines.iter().enumerate() {
        let code = format!("P-{index}");
        let id = till_product(
            &state,
            &code,
            &code,
            *price,
            catalog::UNIT_PIECE,
            false,
            None,
        );
        stock_in(&state, &id, Qty(100_000));
        till::checkout_scan(&state, &format!("{qty}*{code}")).expect("scan");
        products.push(id);
    }
    if plan.discount_minor > 0 {
        let discount = Some(ManualDiscount::Amount(Money(plan.discount_minor)));
        // A discount bigger than the sale is refused: then the sale has none.
        till::checkout_set_invoice_discount(&state, discount, None).ok();
    }
    let total = till::checkout_state(&state)
        .expect("till")
        .cart
        .expect("cart")
        .totals
        .total;
    let sale = till::checkout_complete(&state, devices(), &[cash(&state, total.0)]).expect("sold");
    (folder, state, products, sale.sale_id)
}

fn on_hand(state: &AppState, products: &[Id]) -> Vec<i64> {
    state
        .db
        .read(|conn| {
            let mut found = Vec::new();
            for product in products {
                found.push(inventory::on_hand(conn, product)?.0);
            }
            Ok(found)
        })
        .expect("stock")
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    /// Rule RET-02: however a sale is returned, the returns add up to it exactly.
    #[test]
    fn ret_02_returns_in_any_pieces_add_up_to_the_sale_exactly(
        plan in sale_plan(),
        picks in prop::collection::vec((0usize..4, 1i64..4, any::<bool>()), 0..10),
    ) {
        let (_folder, state, products, sale_id) = sold(&plan);
        let cash_id = cash_method(&state);
        let stored = state.db.read(|conn| Ok(sales::get_sale(conn, &sale_id)?)).expect("sale");
        let mut left: Vec<i64> = stored.lines.iter().map(|line| line.snapshot.qty.0).collect();
        let mut return_ids = Vec::new();
        let mut written_off = vec![0i64; products.len()];
        let send = |line: usize, qty: i64, restock: bool| {
            let input = ReturnInput {
                sale_id: Some(sale_id.clone()),
                lines: vec![ReturnLineInput {
                    sale_line_id: Some(stored.lines[line].id.clone()),
                    product_id: None,
                    qty: Qty(qty),
                    restock,
                    unit_price: None,
                }],
                reason: Some("Changed mind".into()),
                refund_method_id: cash_id.clone(),
                reference: None,
            };
            return_commands::return_complete(&state, devices(), &input, None).expect("returned")
        };
        // Random pieces first, some written off as damaged …
        for (line, pieces, restock) in picks {
            let line = line % left.len();
            let qty = (pieces * 1_000).min(left[line]);
            if qty == 0 {
                continue;
            }
            return_ids.push(send(line, qty, restock).return_id);
            left[line] -= qty;
            if !restock {
                written_off[line] += qty;
            }
        }
        // … then whatever is left.
        for (line, qty) in left.iter().enumerate() {
            if *qty > 0 {
                return_ids.push(send(line, *qty, true).return_id);
            }
        }
        let (mut net, mut tax, mut gross, mut cost) = (0, 0, 0, 0);
        for id in &return_ids {
            let found = state.db.read(|conn| Ok(sales::get_return(conn, id)?)).expect("return");
            net += found.totals.net.0;
            tax += found.totals.tax.0;
            gross += found.totals.gross.0;
            cost += found.totals.cost.0;
        }
        prop_assert_eq!(net, stored.totals.net.0);
        prop_assert_eq!(tax, stored.totals.tax.0);
        prop_assert_eq!(gross, stored.totals.gross.0);
        prop_assert_eq!(cost, stored.totals.cost.0);
        let stock = on_hand(&state, &products);
        for (index, level) in stock.iter().enumerate() {
            prop_assert_eq!(*level, 100_000 - written_off[index]);
        }
        let mismatches = state.db.read(|conn| Ok(inventory::verify(conn)?)).expect("verify");
        prop_assert!(mismatches.is_empty());
        let after = return_commands::return_lookup(&state, &stored.invoice_no).expect("found");
        prop_assert!(after.lines.iter().all(|line| line.returnable.0 == 0));
    }

    /// Rule RET-04: a void puts back every bit of stock and money the sale moved.
    #[test]
    fn ret_04_a_void_reverses_the_sale_exactly(plan in sale_plan()) {
        let (_folder, state, products, sale_id) = sold(&plan);
        let shift_id = state
            .db
            .read(|conn| Ok(app::workflows::till::till_shift(conn)?.expect("open").id))
            .expect("shift");
        return_commands::sale_void(&state, devices(), &sale_id, "Test", None).expect("voided");
        prop_assert!(on_hand(&state, &products).iter().all(|level| *level == 100_000));
        let drawer = state
            .db
            .read(|conn| Ok(shifts::expected_cash(conn, &shift_id)?))
            .expect("drawer");
        prop_assert_eq!(drawer, Money::zero());
        let mismatches = state.db.read(|conn| Ok(inventory::verify(conn)?)).expect("verify");
        prop_assert!(mismatches.is_empty());
    }
}
