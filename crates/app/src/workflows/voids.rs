//! Voiding a completed sale (RET-04; Workflows "Void sale"). Allowed only while the
//! shift that made the sale is still open; after that a return is used. One
//! transaction reverses everything the sale did: stock (`sale_void` rows on the same
//! batches), the shift's payments, the customer's points and credit. The sale keeps its
//! number and contents, marked VOID. The void slip prints after the commit.

use pos_core::{AppError, Id, Money, RoundingRule, cash_round};
use sales::{PaymentKind, SaleStatus};
use serde::Serialize;
use shifts::ShiftsError;
use ts_rs::TS;

use crate::audit_actions::SALE_VOID;
use crate::audit_log;
use crate::devices::Devices;
use crate::state::AppState;
use crate::workflows::customer_records::ledger_ctx;
use crate::workflows::receipts::{self, PrintResult};
use crate::workflows::returns::{paid_with_kind, record_cash_refund};
use crate::workflows::stock::move_ctx;
use crate::workflows::till::{conflict, till_shift};

/// A voided sale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct VoidDone {
    pub sale_id: Id,
    /// Example: "T1-000123". The sale keeps it.
    pub invoice_no: String,
    /// Cash to hand back: what the sale took in cash, and an exchange's credit.
    pub cash_back: Money,
    /// How the void slip printed. None only inside `void`, before printing.
    pub receipt: Option<PrintResult>,
}

/// Voids the sale in one transaction. `approved_by` is the admin who allowed it (USR-05).
pub fn void(
    state: &AppState,
    user: &Id,
    approved_by: Option<&Id>,
    sale_id: &Id,
    reason: &str,
) -> Result<VoidDone, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let sale = sales::get_sale(tx, sale_id)?;
        if sale.status == SaleStatus::Voided {
            return Err(conflict("sale_already_voided"));
        }
        let shift_open = till_shift(tx)?.is_some_and(|shift| shift.id == sale.shift_id);
        if !shift_open {
            return Err(conflict("sale_shift_closed"));
        }
        let now = clock.now();
        sales::void(tx, &sale.id, user, approved_by, reason, now)?;
        let stock_ctx = move_ctx(tx, clock, user)?;
        inventory::reverse(
            tx,
            "sale",
            &sale.id,
            inventory::MoveKind::SaleVoid,
            &stock_ctx,
        )?;
        match shifts::reverse_ref(tx, &sale.shift_id, "sale", &sale.id, user, now) {
            // A sale of nothing (everything free) moved no money.
            Ok(()) | Err(ShiftsError::NothingToReverse { .. }) => {}
            Err(other) => return Err(other.into()),
        }
        customers::reverse_ref(tx, "sale", &sale.id, &ledger_ctx(tx, clock, user)?)?;
        let mut cash_back = paid_with_kind(&sale.payments, PaymentKind::Cash)?;
        // The new half of an exchange was paid with the return's credit: that credit
        // goes back in cash, so the customer ends up with their refund.
        if let Some(return_id) = &sale.exchange_return_id {
            let rule: RoundingRule = settings::get(tx)?;
            let credit = paid_with_kind(&sale.payments, PaymentKind::ExchangeCredit)?;
            let cash = cash_round(credit, &rule)?;
            record_cash_refund(tx, clock, &sale.shift_id, user, cash, return_id)?;
            cash_back = cash_back.checked_add(cash)?;
        }
        audit_log::record_approved(
            tx,
            clock,
            SALE_VOID,
            serde_json::json!({
                "sale_id": sale.id,
                "invoice_no": sale.invoice_no,
                "total_minor": sale.totals.total.0,
                "reason": reason.trim(),
            }),
            Some(user.clone()),
            approved_by.cloned(),
        )?;
        Ok(VoidDone {
            sale_id: sale.id,
            invoice_no: sale.invoice_no,
            cash_back,
            receipt: None,
        })
    })
}

/// Voids the sale, then prints the void slip and opens the drawer when cash goes back.
/// A printer problem never undoes the void (HW-08).
pub fn void_and_print(
    state: &AppState,
    devices: &Devices,
    user: &Id,
    approved_by: Option<&Id>,
    sale_id: &Id,
    reason: &str,
) -> Result<VoidDone, AppError> {
    let mut done = void(state, user, approved_by, sale_id, reason)?;
    done.receipt = Some(receipts::print_receipt(
        state,
        devices,
        &done.sale_id,
        false,
        done.cash_back.0 > 0,
    ));
    Ok(done)
}
