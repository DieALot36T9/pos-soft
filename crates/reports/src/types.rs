//! The inputs the report functions take. `app` fills them in.

use std::collections::BTreeMap;

use pos_core::{BusinessDate, Id};

use crate::ReportsError;

/// The audit action for a cancelled cart (CHK-05). `app::audit_actions` must use the same text.
pub const ACTION_CART_CANCEL: &str = "cart.cancel";
/// The audit action for a voided sale (RET-04).
pub const ACTION_SALE_VOID: &str = "sale.void";
/// The audit action for a no-sale drawer opening (SHF-04).
pub const ACTION_DRAWER_NO_SALE: &str = "drawer.no_sale";

/// Business dates from `from` to `to`, both included. Example: 2026-03-01 to 2026-03-31.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateRange {
    pub from: BusinessDate,
    pub to: BusinessDate,
}

impl DateRange {
    /// Refuses a range that ends before it starts.
    pub(crate) fn check(self) -> Result<DateRange, ReportsError> {
        if self.to < self.from {
            return Err(ReportsError::Invalid("date_range_backwards"));
        }
        Ok(self)
    }
}

/// Names and the time zone the reports need but don't own. `app` copies them from
/// `auth` (users), `settings` (tills, time zone) and `sales` (payment methods).
/// A missing name shows the id instead, so a report never fails over a name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShopContext {
    pub user_names: BTreeMap<Id, String>,
    pub till_names: BTreeMap<Id, String>,
    pub payment_method_names: BTreeMap<Id, String>,
    /// Minutes between local time and UTC. Dhaka is +360. Used for hours and clock times.
    pub utc_offset_minutes: i32,
}

impl ShopContext {
    /// The user's name, or the id text when `app` didn't pass one.
    pub(crate) fn user_name(&self, user_id: &str) -> String {
        name_from(&self.user_names, user_id)
    }

    pub(crate) fn till_name(&self, till_id: &str) -> String {
        name_from(&self.till_names, till_id)
    }

    /// The payment method's name, or its kind in plain words. Example: "Card".
    pub(crate) fn payment_method_name(&self, method_id: &str, kind: &str) -> String {
        let found = Id::parse(method_id)
            .ok()
            .and_then(|id| self.payment_method_names.get(&id).cloned());
        found.unwrap_or_else(|| kind_name(kind))
    }
}

fn name_from(names: &BTreeMap<Id, String>, id_text: &str) -> String {
    Id::parse(id_text)
        .ok()
        .and_then(|id| names.get(&id).cloned())
        .unwrap_or_else(|| id_text.to_string())
}

/// "exchange_credit" → "Exchange credit".
fn kind_name(kind: &str) -> String {
    let words = kind.replace('_', " ");
    let mut letters = words.chars();
    match letters.next() {
        Some(first) => first.to_uppercase().chain(letters).collect(),
        None => String::new(),
    }
}

/// What `sales_by` groups the sales by (RPT-03).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SalesDimension {
    Product,
    Category,
    Brand,
    Cashier,
    /// Hour of the day, in local time. Example: "15:00" holds 15:00 to 15:59.
    Hour,
    /// Day of the week of the business date. Example: "Thursday".
    Weekday,
    Date,
}

/// What profit and stock value reports group by (RPT-04, RPT-05).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupBy {
    Product,
    Category,
    Brand,
}

impl From<GroupBy> for SalesDimension {
    fn from(group: GroupBy) -> SalesDimension {
        match group {
            GroupBy::Product => SalesDimension::Product,
            GroupBy::Category => SalesDimension::Category,
            GroupBy::Brand => SalesDimension::Brand,
        }
    }
}

/// Optional limits for `sales_by` (RPT-03). `Default` means "everything".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SalesFilters {
    pub category_id: Option<Id>,
    pub brand_id: Option<Id>,
    pub cashier_id: Option<Id>,
    /// Keep only the first N rows after sorting. The totals still cover every row.
    pub top: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_name_shows_the_id_or_the_kind() {
        let shop = ShopContext::default();
        assert_eq!(shop.user_name("abc"), "abc");
        assert_eq!(
            shop.payment_method_name("abc", "exchange_credit"),
            "Exchange credit"
        );
    }

    #[test]
    fn a_backwards_range_is_refused() {
        let range = DateRange {
            from: BusinessDate::parse("2026-03-02").unwrap(),
            to: BusinessDate::parse("2026-03-01").unwrap(),
        };
        assert_eq!(
            range.check(),
            Err(ReportsError::Invalid("date_range_backwards"))
        );
    }
}
