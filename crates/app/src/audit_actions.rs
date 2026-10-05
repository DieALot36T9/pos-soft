//! Every audit action name, in one place (Module - audit). Each is lower-case words
//! joined by dots, as `audit::record` requires. Later milestones add the rest.

/// A cart was cancelled at the till (CHK-05).
pub const CART_CANCEL: &str = "cart.cancel";
/// A completed sale was voided (RET-04).
pub const SALE_VOID: &str = "sale.void";
/// The drawer was opened without a sale (SHF-04).
pub const DRAWER_NO_SALE: &str = "drawer.no_sale";
/// A setting was changed after setup; details hold the old and new values (SET-13).
pub const SETTINGS_CHANGE: &str = "settings.change";
/// A tax rate was added, changed or deactivated after setup (TAX-02, TAX-10).
pub const TAX_RATE_CHANGE: &str = "tax_rate.change";
/// Someone pressed "Back up now" (BAK-04).
pub const BACKUP_MANUAL: &str = "backup.manual";
/// A backup was restored. Written into the restored database after the restart (BAK-05).
pub const BACKUP_RESTORE: &str = "backup.restore";
/// Detailed logging was switched on or off (BAK-07).
pub const LOGGING_SWITCH: &str = "logging.switch";

#[cfg(test)]
mod tests {
    use super::*;

    /// The reports count these actions by name (RPT-01, RPT-08, SHF-08). If a name
    /// changed here only, those counts would quietly stay at 0.
    #[test]
    fn reports_count_the_same_action_names() {
        assert_eq!(CART_CANCEL, reports::ACTION_CART_CANCEL);
        assert_eq!(SALE_VOID, reports::ACTION_SALE_VOID);
        assert_eq!(DRAWER_NO_SALE, reports::ACTION_DRAWER_NO_SALE);
    }
}
