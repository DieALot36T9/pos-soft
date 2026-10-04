//! Every permission in the app. The full table with defaults is in
//! `docs/02 Requirements/Roles and Permissions.md`. Which role has which
//! permission is decided in the auth module, not here.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One thing a user may be allowed to do. Stored as snake_case text, e.g. "checkout_sell".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Permission {
    CheckoutSell,
    CheckoutCancelCart,
    CheckoutHoldResume,
    CheckoutManualWeight,
    DiscountApply,
    PriceOverride,
    PriceListSwitch,
    SellOnCredit,
    CreditLimitExceed,
    ReceiptReprintLast,
    ReceiptReprintAny,
    SaleVoid,
    ReturnWithReceipt,
    ReturnWithoutReceipt,
    ShiftOpenCloseOwn,
    ShiftCashInOut,
    ShiftNoSaleDrawer,
    ShiftForceClose,
    CustomerViewCreate,
    CustomerManage,
    CustomerReceivePayment,
    LoyaltyAdjust,
    StockView,
    StockAdjust,
    StockCountEnter,
    StockCountManage,
    CatalogManage,
    CatalogPriceEdit,
    PricingManage,
    TaxManage,
    PurchasingManage,
    PurchasingPay,
    LabelsPrint,
    ReportsViewAll,
    DataImport,
    DataExport,
    UsersManage,
    SettingsManage,
    BackupRun,
    BackupRestore,
    AuditView,
    LogsManage,
}

impl Permission {
    /// Every permission, in the order of the Roles and Permissions page.
    pub const ALL: [Permission; 42] = [
        Permission::CheckoutSell,
        Permission::CheckoutCancelCart,
        Permission::CheckoutHoldResume,
        Permission::CheckoutManualWeight,
        Permission::DiscountApply,
        Permission::PriceOverride,
        Permission::PriceListSwitch,
        Permission::SellOnCredit,
        Permission::CreditLimitExceed,
        Permission::ReceiptReprintLast,
        Permission::ReceiptReprintAny,
        Permission::SaleVoid,
        Permission::ReturnWithReceipt,
        Permission::ReturnWithoutReceipt,
        Permission::ShiftOpenCloseOwn,
        Permission::ShiftCashInOut,
        Permission::ShiftNoSaleDrawer,
        Permission::ShiftForceClose,
        Permission::CustomerViewCreate,
        Permission::CustomerManage,
        Permission::CustomerReceivePayment,
        Permission::LoyaltyAdjust,
        Permission::StockView,
        Permission::StockAdjust,
        Permission::StockCountEnter,
        Permission::StockCountManage,
        Permission::CatalogManage,
        Permission::CatalogPriceEdit,
        Permission::PricingManage,
        Permission::TaxManage,
        Permission::PurchasingManage,
        Permission::PurchasingPay,
        Permission::LabelsPrint,
        Permission::ReportsViewAll,
        Permission::DataImport,
        Permission::DataExport,
        Permission::UsersManage,
        Permission::SettingsManage,
        Permission::BackupRun,
        Permission::BackupRestore,
        Permission::AuditView,
        Permission::LogsManage,
    ];

    /// The text stored in the database for this permission. Example: "checkout_sell".
    pub fn as_str(self) -> &'static str {
        match self {
            Permission::CheckoutSell => "checkout_sell",
            Permission::CheckoutCancelCart => "checkout_cancel_cart",
            Permission::CheckoutHoldResume => "checkout_hold_resume",
            Permission::CheckoutManualWeight => "checkout_manual_weight",
            Permission::DiscountApply => "discount_apply",
            Permission::PriceOverride => "price_override",
            Permission::PriceListSwitch => "price_list_switch",
            Permission::SellOnCredit => "sell_on_credit",
            Permission::CreditLimitExceed => "credit_limit_exceed",
            Permission::ReceiptReprintLast => "receipt_reprint_last",
            Permission::ReceiptReprintAny => "receipt_reprint_any",
            Permission::SaleVoid => "sale_void",
            Permission::ReturnWithReceipt => "return_with_receipt",
            Permission::ReturnWithoutReceipt => "return_without_receipt",
            Permission::ShiftOpenCloseOwn => "shift_open_close_own",
            Permission::ShiftCashInOut => "shift_cash_in_out",
            Permission::ShiftNoSaleDrawer => "shift_no_sale_drawer",
            Permission::ShiftForceClose => "shift_force_close",
            Permission::CustomerViewCreate => "customer_view_create",
            Permission::CustomerManage => "customer_manage",
            Permission::CustomerReceivePayment => "customer_receive_payment",
            Permission::LoyaltyAdjust => "loyalty_adjust",
            Permission::StockView => "stock_view",
            Permission::StockAdjust => "stock_adjust",
            Permission::StockCountEnter => "stock_count_enter",
            Permission::StockCountManage => "stock_count_manage",
            Permission::CatalogManage => "catalog_manage",
            Permission::CatalogPriceEdit => "catalog_price_edit",
            Permission::PricingManage => "pricing_manage",
            Permission::TaxManage => "tax_manage",
            Permission::PurchasingManage => "purchasing_manage",
            Permission::PurchasingPay => "purchasing_pay",
            Permission::LabelsPrint => "labels_print",
            Permission::ReportsViewAll => "reports_view_all",
            Permission::DataImport => "data_import",
            Permission::DataExport => "data_export",
            Permission::UsersManage => "users_manage",
            Permission::SettingsManage => "settings_manage",
            Permission::BackupRun => "backup_run",
            Permission::BackupRestore => "backup_restore",
            Permission::AuditView => "audit_view",
            Permission::LogsManage => "logs_manage",
        }
    }

    /// Reads a permission from its stored text. Unknown text gives `None`.
    pub fn parse(text: &str) -> Option<Permission> {
        Permission::ALL
            .into_iter()
            .find(|permission| permission.as_str() == text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_text_matches_what_the_ui_receives() {
        for permission in Permission::ALL {
            let json = serde_json::to_string(&permission).unwrap();
            assert_eq!(json, format!("\"{}\"", permission.as_str()));
        }
    }

    #[test]
    fn every_permission_reads_back() {
        for permission in Permission::ALL {
            assert_eq!(Permission::parse(permission.as_str()), Some(permission));
        }
        assert_eq!(Permission::parse("fly_to_the_moon"), None);
    }

    #[test]
    fn all_lists_each_permission_once() {
        let mut texts: Vec<&str> = Permission::ALL.iter().map(|p| p.as_str()).collect();
        texts.sort_unstable();
        texts.dedup();
        assert_eq!(texts.len(), Permission::ALL.len());
    }
}
