//! Tauri commands, one file per area of the UI. Commands stay thin: check the
//! permission, call one workflow or query, return.

pub mod audit;
pub mod auth;
pub mod backup;
pub mod catalog;
pub mod checkout;
pub mod customers;
pub mod dev;
pub mod hardware;
pub mod inventory;
pub mod io;
pub mod labels;
pub mod payment_methods;
pub mod pricing;
pub mod purchasing;
pub mod receipts;
pub mod reports;
pub mod returns;
pub mod settings;
pub mod setup;
pub mod shifts;
pub mod startup;
pub mod system;
pub mod tax;
pub mod users;

use pos_core::{AppError, Permission};

/// Every registered command and the permission it needs (`None` = anyone, even before login).
/// A new command must be added here and to `generate_handler!` in `lib.rs` (Security Model).
///
/// Some commands have their own rule instead of a permission:
/// - `setup_*` work only while setup isn't complete (nobody exists yet to sign in).
/// - `recovery_*` work only while the database is damaged (BAK-06).
/// - `settings_get`, `settings_save` and `tax_rate_*` also work without sign-in
///   while setup is open, because the wizard uses them.
/// - `hardware_test_print` uses the saved printer, so it needs `SettingsManage`; it
///   also works without sign-in while setup is open (the wizard's hardware step).
/// - `print_retry`, `print_system_pictures` and `print_dismiss` need someone signed in
///   (any role), and work only for a printout that failed (`print_not_waiting`).
/// - `shift_x_report` prints only the user's own open shift (`shift_not_yours`).
/// - `login_tiles`, `login` and `recover_admin` work only once setup is complete.
/// - `lock`, `unlock`, `unlock_by_admin`, `request_override` and `change_own_secret`
///   need someone signed in (any role); `unlock*` only while the till is locked.
/// - `backup_restore` also takes an admin's approval token (`require_or_approval`, USR-05).
/// - `product_save` also needs `CatalogPriceEdit` when it changes a sale or cost price.
/// - `checkout_add_weighed` with a typed weight also needs `CheckoutManualWeight`.
/// - `shift_current` and `shift_count_total` need someone signed in (any role).
/// - `shift_cash_move`, `shift_no_sale` and `shift_force_close` also take an admin's
///   approval token (`require_or_approval`, USR-05).
/// - `checkout_set_discount` and `checkout_set_invoice_discount` take an admin's
///   approval token (`DiscountApply`) for a discount above the role's limit (PRC-07).
/// - `checkout_price_override` and `checkout_set_price_list` also need `CheckoutSell`, and
///   take an approval token (`require_or_approval`). `checkout_set_customer` also needs
///   `CheckoutSell`. `checkout_complete` also needs `SellOnCredit` to pay with customer
///   credit, and takes an approval token for credit over the limit (`CreditLimitExceed`).
/// - `customer_save` also needs `CustomerManage` to change a customer that exists.
/// - `return_complete` and `exchange_start` need `ReturnWithoutReceipt` instead for a
///   return without a receipt; `exchange_start` also needs `CheckoutSell`. Both, and
///   `sale_void`, take an admin's approval token (`require_or_approval`, USR-05). A typed
///   price on a return without a receipt also needs `PriceOverride` (the user's or the
///   approving admin's).
/// - `report_run`, `report_export_xlsx` and `report_print` also let a user with
///   `ShiftOpenCloseOwn` see the X or Z report of their own shift (RPT-10).
///   `report_shifts` lists everyone's shifts with `ReportsViewAll`, otherwise only the
///   user's own.
/// - `import_commit` makes a `pre-import` backup first, without `BackupRun`, like
///   `count_post`. `io_price_lists` also works with `DataExport` instead of `DataImport`.
/// - `count_post` makes a `pre-count-post` backup first, without `BackupRun`: the backup
///   is part of posting (Workflows, "Post stock count").
pub const COMMANDS: &[(&str, Option<Permission>)] = &[
    ("app_info", None),
    ("hardware_test_print", Some(Permission::SettingsManage)),
    ("preview_take_printouts", None),
    ("print_window", None),
    ("startup_status", None),
    ("session_current", None),
    ("login_tiles", None),
    ("login", None),
    ("logout", None),
    ("lock", None),
    ("unlock", None),
    ("unlock_by_admin", None),
    ("request_override", None),
    ("change_own_secret", None),
    ("recover_admin", None),
    ("recovery_backups", None),
    ("recovery_restore", None),
    ("setup_status", None),
    ("setup_save_shop", None),
    ("setup_save_money", None),
    ("setup_save_tax", None),
    ("setup_save_admin", None),
    ("setup_save_till", None),
    ("setup_save_hardware", None),
    ("setup_save_backup", None),
    ("setup_finish", None),
    ("setup_restore_backup", None),
    ("currency_choices", None),
    ("currency_format_for", None),
    ("money_preview", None),
    ("settings_get", Some(Permission::SettingsManage)),
    ("settings_save", Some(Permission::SettingsManage)),
    ("logs_set_detailed", Some(Permission::LogsManage)),
    ("tax_rate_list", Some(Permission::TaxManage)),
    ("tax_rate_save", Some(Permission::TaxManage)),
    ("tax_rate_deactivate", Some(Permission::TaxManage)),
    ("backup_now", Some(Permission::BackupRun)),
    ("backup_list", Some(Permission::BackupRun)),
    ("backup_restore", Some(Permission::BackupRestore)),
    ("user_list", Some(Permission::UsersManage)),
    ("user_save", Some(Permission::UsersManage)),
    ("role_list", Some(Permission::UsersManage)),
    ("role_save", Some(Permission::UsersManage)),
    ("audit_query", Some(Permission::AuditView)),
    ("audit_export", Some(Permission::AuditView)),
    ("product_list", Some(Permission::StockView)),
    ("product_search", Some(Permission::StockView)),
    ("product_get", Some(Permission::StockView)),
    ("product_choices", Some(Permission::StockView)),
    ("product_save", Some(Permission::CatalogManage)),
    ("product_archive", Some(Permission::CatalogManage)),
    ("product_unarchive", Some(Permission::CatalogManage)),
    ("product_delete", Some(Permission::CatalogManage)),
    ("barcode_generate", Some(Permission::CatalogManage)),
    ("variants_generate", Some(Permission::CatalogManage)),
    ("kit_save", Some(Permission::CatalogManage)),
    ("product_image_set", Some(Permission::CatalogManage)),
    ("unit_list", Some(Permission::StockView)),
    ("unit_save", Some(Permission::CatalogManage)),
    ("unit_archive", Some(Permission::CatalogManage)),
    ("category_list", Some(Permission::StockView)),
    ("category_save", Some(Permission::CatalogManage)),
    ("category_archive", Some(Permission::CatalogManage)),
    ("brand_list", Some(Permission::StockView)),
    ("brand_save", Some(Permission::CatalogManage)),
    ("brand_archive", Some(Permission::CatalogManage)),
    ("stock_levels", Some(Permission::StockView)),
    ("stock_product", Some(Permission::StockView)),
    ("stock_lookup", Some(Permission::StockView)),
    ("stock_history", Some(Permission::StockView)),
    ("stock_reasons", Some(Permission::StockView)),
    ("stock_adjust", Some(Permission::StockAdjust)),
    ("low_stock", Some(Permission::StockView)),
    ("near_expiry", Some(Permission::StockView)),
    ("stock_alerts", Some(Permission::StockView)),
    ("stock_verify", Some(Permission::StockAdjust)),
    ("count_list", Some(Permission::StockCountEnter)),
    ("count_get", Some(Permission::StockCountEnter)),
    ("count_create", Some(Permission::StockCountManage)),
    ("count_line", Some(Permission::StockCountEnter)),
    ("count_variances", Some(Permission::StockCountManage)),
    ("count_post", Some(Permission::StockCountManage)),
    ("count_cancel", Some(Permission::StockCountManage)),
    ("checkout_state", Some(Permission::CheckoutSell)),
    ("checkout_scan", Some(Permission::CheckoutSell)),
    ("checkout_search", Some(Permission::CheckoutSell)),
    ("checkout_add_product", Some(Permission::CheckoutSell)),
    ("checkout_read_scale", Some(Permission::CheckoutSell)),
    ("checkout_add_weighed", Some(Permission::CheckoutSell)),
    ("checkout_set_qty", Some(Permission::CheckoutSell)),
    ("checkout_remove_line", Some(Permission::CheckoutSell)),
    ("checkout_hold", Some(Permission::CheckoutHoldResume)),
    ("checkout_held_list", Some(Permission::CheckoutHoldResume)),
    ("checkout_resume", Some(Permission::CheckoutHoldResume)),
    ("checkout_cancel", Some(Permission::CheckoutCancelCart)),
    ("checkout_payment_preview", Some(Permission::CheckoutSell)),
    ("checkout_complete", Some(Permission::CheckoutSell)),
    ("shift_current", None),
    ("shift_count_total", None),
    ("shift_open", Some(Permission::ShiftOpenCloseOwn)),
    ("shift_cash_move", Some(Permission::ShiftCashInOut)),
    ("shift_no_sale", Some(Permission::ShiftNoSaleDrawer)),
    ("shift_close", Some(Permission::ShiftOpenCloseOwn)),
    ("shift_force_close", Some(Permission::ShiftForceClose)),
    ("shift_x_report", Some(Permission::ShiftOpenCloseOwn)),
    ("receipt_reprint_last", Some(Permission::ReceiptReprintLast)),
    ("receipt_reprint", Some(Permission::ReceiptReprintAny)),
    ("print_retry", None),
    ("print_system_pictures", None),
    ("print_dismiss", None),
    ("payment_method_list", Some(Permission::SettingsManage)),
    ("payment_method_save", Some(Permission::SettingsManage)),
    ("checkout_set_discount", Some(Permission::CheckoutSell)),
    (
        "checkout_set_invoice_discount",
        Some(Permission::CheckoutSell),
    ),
    ("checkout_price_override", Some(Permission::PriceOverride)),
    (
        "checkout_set_customer",
        Some(Permission::CustomerViewCreate),
    ),
    ("checkout_set_price_list", Some(Permission::PriceListSwitch)),
    ("price_list_list", Some(Permission::PricingManage)),
    ("price_list_save", Some(Permission::PricingManage)),
    ("price_list_items", Some(Permission::PricingManage)),
    ("price_list_set_items", Some(Permission::PricingManage)),
    ("price_list_remove_items", Some(Permission::PricingManage)),
    ("tier_list", Some(Permission::PricingManage)),
    ("tier_save", Some(Permission::PricingManage)),
    ("tier_archive", Some(Permission::PricingManage)),
    ("promotion_list", Some(Permission::PricingManage)),
    ("promotion_save", Some(Permission::PricingManage)),
    ("promotion_set_active", Some(Permission::PricingManage)),
    ("customer_lookup", Some(Permission::CustomerViewCreate)),
    ("customer_list", Some(Permission::CustomerViewCreate)),
    ("customer_get", Some(Permission::CustomerViewCreate)),
    ("customer_save", Some(Permission::CustomerViewCreate)),
    ("customer_archive", Some(Permission::CustomerManage)),
    ("customer_payment", Some(Permission::CustomerReceivePayment)),
    ("points_adjust", Some(Permission::LoyaltyAdjust)),
    ("customer_statement", Some(Permission::CustomerViewCreate)),
    ("return_lookup", Some(Permission::CheckoutSell)),
    ("return_preview", Some(Permission::CheckoutSell)),
    ("return_complete", Some(Permission::ReturnWithReceipt)),
    ("exchange_start", Some(Permission::ReturnWithReceipt)),
    ("exchange_cash_out", Some(Permission::CheckoutSell)),
    ("sale_void", Some(Permission::SaleVoid)),
    ("supplier_list", Some(Permission::PurchasingManage)),
    ("supplier_get", Some(Permission::PurchasingManage)),
    ("supplier_save", Some(Permission::PurchasingManage)),
    ("supplier_archive", Some(Permission::PurchasingManage)),
    ("po_list", Some(Permission::PurchasingManage)),
    ("po_get", Some(Permission::PurchasingManage)),
    ("po_save", Some(Permission::PurchasingManage)),
    ("po_suggest", Some(Permission::PurchasingManage)),
    ("po_mark_sent", Some(Permission::PurchasingManage)),
    ("po_export", Some(Permission::PurchasingManage)),
    ("po_cancel", Some(Permission::PurchasingManage)),
    ("grn_start", Some(Permission::PurchasingManage)),
    ("grn_preview", Some(Permission::PurchasingManage)),
    ("grn_save", Some(Permission::PurchasingManage)),
    ("grn_list", Some(Permission::PurchasingManage)),
    ("grn_get", Some(Permission::PurchasingManage)),
    ("supplier_return_save", Some(Permission::PurchasingManage)),
    ("supplier_return_list", Some(Permission::PurchasingManage)),
    ("payables_aging", Some(Permission::PurchasingManage)),
    ("supplier_statement", Some(Permission::PurchasingManage)),
    (
        "supplier_statement_export",
        Some(Permission::PurchasingManage),
    ),
    ("supplier_payment", Some(Permission::PurchasingPay)),
    ("report_choices", Some(Permission::ReportsViewAll)),
    ("report_customer_search", Some(Permission::ReportsViewAll)),
    ("report_shifts", Some(Permission::ShiftOpenCloseOwn)),
    ("report_run", Some(Permission::ReportsViewAll)),
    ("report_export_xlsx", Some(Permission::ReportsViewAll)),
    ("report_print", Some(Permission::ReportsViewAll)),
    ("import_preview", Some(Permission::DataImport)),
    ("import_commit", Some(Permission::DataImport)),
    ("import_progress", Some(Permission::DataImport)),
    ("import_cancel", Some(Permission::DataImport)),
    ("import_errors_save", Some(Permission::DataImport)),
    ("template_download", Some(Permission::DataImport)),
    ("export_entity", Some(Permission::DataExport)),
    ("io_price_lists", Some(Permission::DataImport)),
    ("labels_pick", Some(Permission::LabelsPrint)),
    ("labels_preview", Some(Permission::LabelsPrint)),
    ("labels_print", Some(Permission::LabelsPrint)),
];

/// The dev-only commands (fake scale panel). Registered only when `dev_tools::ON` is true.
/// They need no permission: they only change the pretend scale.
pub const DEV_COMMANDS: &[(&str, Option<Permission>)] = &[
    ("dev_fake_scale_get", None),
    ("dev_fake_scale_set", None),
    ("dev_fake_scale_read", None),
];

/// Runs a command's work on a worker thread, so the window never waits, inside a
/// log span named after the command. A panic becomes `AppError::Internal`, and an
/// internal error is logged with its details (the screen shows only its reference).
pub(crate) async fn on_worker<T, F>(name: &'static str, work: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, AppError> + Send + 'static,
{
    let result = tauri::async_runtime::spawn_blocking(move || {
        let span = tracing::info_span!("cmd", name);
        let _entered = span.enter();
        let started = std::time::Instant::now();
        let result = work();
        tracing::info!(
            ms = started.elapsed().as_millis() as u64,
            ok = result.is_ok(),
            "done"
        );
        result
    })
    .await
    .map_err(|error| AppError::internal(format!("{name} stopped: {error}")))?;
    if let Err(AppError::Internal { id, detail }) = &result {
        tracing::error!(command = name, id, detail, "internal error");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[(&'static str, Option<Permission>)]) -> Vec<&'static str> {
        list.iter().map(|(name, _)| *name).collect()
    }

    #[test]
    fn every_command_has_a_permission_decision() {
        let all = names(COMMANDS);
        assert_eq!(all.len(), 181);
        let mut unique = all.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), all.len(), "a command is listed twice");
        assert_eq!(
            names(DEV_COMMANDS),
            vec![
                "dev_fake_scale_get",
                "dev_fake_scale_set",
                "dev_fake_scale_read"
            ]
        );
    }

    #[test]
    fn app_info_needs_no_login() {
        assert_eq!(COMMANDS[0], ("app_info", None));
    }

    #[test]
    fn set_13_settings_tax_backup_and_logs_need_their_permission() {
        let needs = |name: &str| {
            COMMANDS
                .iter()
                .find(|(other, _)| *other == name)
                .map(|(_, p)| *p)
        };
        assert_eq!(
            needs("settings_save"),
            Some(Some(Permission::SettingsManage))
        );
        assert_eq!(needs("tax_rate_save"), Some(Some(Permission::TaxManage)));
        assert_eq!(needs("backup_now"), Some(Some(Permission::BackupRun)));
        assert_eq!(
            needs("backup_restore"),
            Some(Some(Permission::BackupRestore))
        );
        assert_eq!(
            needs("logs_set_detailed"),
            Some(Some(Permission::LogsManage))
        );
    }

    #[test]
    fn hw_10_printing_without_a_device_needs_no_login() {
        for name in ["preview_take_printouts", "print_window"] {
            assert!(COMMANDS.contains(&(name, None)), "{name}");
        }
    }

    #[test]
    fn hw_02_test_print_reaches_the_saved_printer_so_needs_settings_manage() {
        assert!(COMMANDS.contains(&("hardware_test_print", Some(Permission::SettingsManage))));
    }

    #[test]
    fn hw_10_dev_commands_are_kept_apart_and_need_no_login() {
        for (name, permission) in DEV_COMMANDS {
            assert!(name.starts_with("dev_"), "{name}");
            assert_eq!(*permission, None, "{name}");
            assert!(!COMMANDS.iter().any(|(other, _)| other == name), "{name}");
        }
    }
}
