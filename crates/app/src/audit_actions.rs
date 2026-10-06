//! Every audit action name, in one place (Module - audit). Each is lower-case words
//! joined by dots, as `audit::record` requires. Later milestones add the rest,
//! and add each one to `ALL`.

/// A cart was cancelled at the till (CHK-05).
pub const CART_CANCEL: &str = "cart.cancel";
/// A completed sale was voided (RET-04).
pub const SALE_VOID: &str = "sale.void";
/// The drawer was opened without a sale (SHF-04).
pub const DRAWER_NO_SALE: &str = "drawer.no_sale";
/// A shift was opened on the till with its float (SHF-01).
pub const SHIFT_OPEN: &str = "shift.open";
/// A cashier closed their shift with the cash count (SHF-06).
pub const SHIFT_CLOSE: &str = "shift.close";
/// An admin closed a shift left open (SHF-07).
pub const SHIFT_FORCE_CLOSE: &str = "shift.force_close";
/// Cash was put into or taken out of the drawer: cash in, paid-out or cash drop (SHF-03).
pub const SHIFT_CASH_MOVE: &str = "shift.cash_move";
/// A payment method was added or changed (SET-15).
pub const PAYMENT_METHOD_CHANGE: &str = "payment_method.change";
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
/// Someone logged in (USR-01).
pub const SESSION_LOGIN: &str = "session.login";
/// Someone logged out, or "Switch user" on the lock screen (USR-01, USR-03).
pub const SESSION_LOGOUT: &str = "session.logout";
/// An admin unlocked the till for the user who locked it (USR-03).
pub const SESSION_UNLOCK_BY_ADMIN: &str = "session.unlock_by_admin";
/// The fifth wrong PIN or password in a row locked a user for 60 seconds (USR-02).
pub const USER_LOCKOUT: &str = "user.lockout";
/// An admin approved one action for the signed-in user (USR-05).
pub const APPROVAL_GRANTED: &str = "approval.granted";
/// A user was added (USR-06).
pub const USER_CREATE: &str = "user.create";
/// A user was renamed, moved to another role, disabled, enabled, or had a PIN or password reset (USR-06).
pub const USER_CHANGE: &str = "user.change";
/// A user changed their own PIN or password (USR-07).
pub const USER_OWN_SECRET_CHANGE: &str = "user.own_secret_change";
/// A custom role was added (USR-04).
pub const ROLE_CREATE: &str = "role.create";
/// A role's permissions or discount limit changed (USR-04).
pub const ROLE_CHANGE: &str = "role.change";
/// The recovery code set a new admin password (USR-09).
pub const RECOVERY_USED: &str = "recovery.used";
/// A product was added (CAT-01).
pub const PRODUCT_CREATE: &str = "product.create";
/// A product's fields, prices, barcodes, variants, kit parts or picture changed (CAT-01 to CAT-11).
pub const PRODUCT_CHANGE: &str = "product.change";
/// A product was archived: hidden from sale and search (CAT-08).
pub const PRODUCT_ARCHIVE: &str = "product.archive";
/// An archived product was brought back (CAT-08).
pub const PRODUCT_UNARCHIVE: &str = "product.unarchive";
/// A product that was never used was deleted for good (CAT-08).
pub const PRODUCT_DELETE: &str = "product.delete";
/// A unit was added, changed or archived (CAT-02).
pub const UNIT_CHANGE: &str = "unit.change";
/// A category was added, changed or archived (CAT-03).
pub const CATEGORY_CHANGE: &str = "category.change";
/// A brand was added, renamed or archived (CAT-03).
pub const BRAND_CHANGE: &str = "brand.change";
/// Stock was adjusted by hand with a reason (INV-02).
pub const STOCK_ADJUST: &str = "stock.adjust";
/// A stock count was started (INV-06).
pub const STOCK_COUNT_CREATE: &str = "stock_count.create";
/// A stock count's differences were posted to stock (INV-06).
pub const STOCK_COUNT_POST: &str = "stock_count.post";
/// A stock count was cancelled; stock didn't change (INV-06).
pub const STOCK_COUNT_CANCEL: &str = "stock_count.cancel";

/// Every action name, for the action filter on the audit screen (USR-08).
pub const ALL: &[&str] = &[
    CART_CANCEL,
    SALE_VOID,
    DRAWER_NO_SALE,
    SHIFT_OPEN,
    SHIFT_CLOSE,
    SHIFT_FORCE_CLOSE,
    SHIFT_CASH_MOVE,
    PAYMENT_METHOD_CHANGE,
    SETTINGS_CHANGE,
    TAX_RATE_CHANGE,
    BACKUP_MANUAL,
    BACKUP_RESTORE,
    LOGGING_SWITCH,
    SESSION_LOGIN,
    SESSION_LOGOUT,
    SESSION_UNLOCK_BY_ADMIN,
    USER_LOCKOUT,
    APPROVAL_GRANTED,
    USER_CREATE,
    USER_CHANGE,
    USER_OWN_SECRET_CHANGE,
    ROLE_CREATE,
    ROLE_CHANGE,
    RECOVERY_USED,
    PRODUCT_CREATE,
    PRODUCT_CHANGE,
    PRODUCT_ARCHIVE,
    PRODUCT_UNARCHIVE,
    PRODUCT_DELETE,
    UNIT_CHANGE,
    CATEGORY_CHANGE,
    BRAND_CHANGE,
    STOCK_ADJUST,
    STOCK_COUNT_CREATE,
    STOCK_COUNT_POST,
    STOCK_COUNT_CANCEL,
];

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

    #[test]
    fn usr_08_every_name_is_listed_once_and_well_formed() {
        let mut names = ALL.to_vec();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), ALL.len(), "a name is listed twice");
        for name in ALL {
            let words_and_dots = name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '_' || c == '.');
            assert!(words_and_dots && name.contains('.'), "{name}");
        }
    }
}
