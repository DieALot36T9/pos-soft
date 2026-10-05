//! PUR-02 "Suggest from low stock": fills a purchase order from the low-stock list.

use pos_core::{Id, Money, Qty};

use crate::NewOrderLine;

/// One product on the low-stock list. `app` builds it from catalog and inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowStockItem {
    pub product_id: Id,
    pub product_name: String,
    pub sku: String,
    pub preferred_supplier_id: Option<Id>,
    /// How much to order when stock is low.
    pub reorder_qty: Qty,
    /// The product's current cost, used as the expected cost.
    pub unit_cost: Money,
}

/// Order lines for the low-stock products whose preferred supplier is `supplier`,
/// each for its reorder quantity. Products with no reorder quantity are skipped.
pub fn suggest_order_lines(supplier: &Id, low_stock: &[LowStockItem]) -> Vec<NewOrderLine> {
    let mut lines = Vec::new();
    for item in low_stock {
        if item.preferred_supplier_id.as_ref() != Some(supplier) || item.reorder_qty.0 <= 0 {
            continue;
        }
        lines.push(NewOrderLine {
            product_id: item.product_id.clone(),
            product_name: item.product_name.clone(),
            sku: item.sku.clone(),
            qty: item.reorder_qty,
            unit_cost: item.unit_cost,
        });
    }
    lines
}
