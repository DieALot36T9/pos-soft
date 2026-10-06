//! Customers at the till and in the back office (Req - Customers and Loyalty): lookup
//! (CUS-02), records (CUS-01), the customer page with purchase history (CUS-03), the
//! credit statement (CUS-05), debt payments (CUS-05) and points corrections (CUS-04).
//! Each save is one transaction with its audit event.

use customers::{
    CreditKind, Customer, CustomerDetails, LedgerCtx, LedgerRef, LoyaltySettingsView, PointsKind,
    Statement,
};
use pos_core::{
    AppError, BusinessDate, Clock, CurrencyFormat, Id, Money, Timestamp, business_date,
};
use rusqlite::{Connection, named_params};
use serde::{Deserialize, Serialize};
use settings::{CreditSettings, LoyaltySettings, TillSettings};
use shifts::{MovementKind, ShiftMovement};
use ts_rs::TS;

use crate::audit_actions::{CUSTOMER_CHANGE, CUSTOMER_CREATE, CUSTOMER_PAYMENT, POINTS_ADJUST};
use crate::audit_log;
use crate::state::AppState;
use crate::workflows::cart_view::running_tier_name;
use crate::workflows::till::{conflict, till_shift};

/// The most customers the till's lookup shows (CUS-02).
const LOOKUP_LIMIT: u32 = 20;
/// The most customers the back-office list shows at once.
const LIST_LIMIT: u32 = 200;
/// The most points rows the customer page shows, newest first.
const POINTS_ROWS: u32 = 100;

/// One customer in a list or the till's lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct CustomerRow {
    pub id: Id,
    pub phone: String,
    pub name: String,
    pub tier_name: Option<String>,
    pub points: i64,
    /// Positive: the customer owes the shop. Negative: store credit.
    pub credit: Money,
    pub archived: bool,
}

/// What the user types for a customer: a new one (`id` empty) or a change.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct CustomerSave {
    pub id: Option<Id>,
    pub details: CustomerDetails,
}

/// One sale or return on the customer page (CUS-03).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct PurchaseRow {
    /// "sale" or "return".
    pub kind: String,
    pub id: Id,
    /// Example: "T1-000123" or "T1-R000045".
    pub number: String,
    pub at: Timestamp,
    /// What the customer paid, or got back for a return.
    pub total: Money,
    pub voided: bool,
}

/// One points ledger row on the customer page (CUS-04).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct PointsRow {
    pub at: Timestamp,
    pub kind: PointsKind,
    pub points: i64,
    pub reason: Option<String>,
}

/// Everything the customer page shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct CustomerPage {
    pub customer: Customer,
    pub tier_name: Option<String>,
    pub points: i64,
    /// Positive: the customer owes the shop. Negative: store credit.
    pub credit: Money,
    pub oldest_unpaid_at: Option<Timestamp>,
    /// Σ completed sales − Σ returns (CUS-03).
    pub total_spend: Money,
    /// Completed sales.
    pub visits: u32,
    /// Newest first.
    pub purchases: Vec<PurchaseRow>,
    /// Newest first, at most 100.
    pub points_history: Vec<PointsRow>,
}

/// A payment against the customer's debt (CUS-05).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct CustomerPaymentInput {
    pub customer_id: Id,
    pub amount: Money,
    /// Cash, card, wallet or another method from Settings.
    pub method_id: Id,
    /// A transaction id or similar (PAY-02).
    pub reference: Option<String>,
}

/// What a debt payment left.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct CustomerPaymentDone {
    /// The new balance. Positive: still owed.
    pub credit: Money,
    /// True when the payment was cash into this user's open shift.
    pub cash_taken: bool,
}

/// Customers whose phone or name matches what was typed, for the till (CUS-02).
/// Archived customers are left out.
pub fn lookup(state: &AppState, typed: &str) -> Result<Vec<CustomerRow>, AppError> {
    state.db.read(|conn| {
        let hits = customers::lookup(conn, typed, LOOKUP_LIMIT)?;
        let mut rows = Vec::with_capacity(hits.len());
        for hit in hits {
            rows.push(CustomerRow {
                tier_name: running_tier_name(conn, hit.tier_id.as_ref())?,
                id: hit.id,
                phone: hit.phone,
                name: hit.name,
                points: hit.points,
                credit: hit.credit,
                archived: false,
            });
        }
        Ok(rows)
    })
}

/// The back-office list: matches for the typed text, or everyone by name when nothing is
/// typed. Archived customers only when asked for.
pub fn list(
    state: &AppState,
    typed: &str,
    include_archived: bool,
) -> Result<Vec<CustomerRow>, AppError> {
    if !typed.trim().is_empty() && !include_archived {
        return lookup(state, typed);
    }
    state.db.read(|conn| {
        let like = format!("%{}%", typed.trim().to_lowercase());
        let mut statement = conn.prepare(
            "SELECT id, phone, name, tier_id, COALESCE(points, 0), COALESCE(credit_minor, 0), archived_at
             FROM customers_v_customers
             WHERE (:all OR archived_at IS NULL)
               AND (lower(name) LIKE :like OR phone LIKE :like)
             ORDER BY lower(name) LIMIT :limit",
        ).map_err(sql_error)?;
        let found = statement
            .query_map(
                named_params! { ":all": include_archived, ":like": like, ":limit": LIST_LIMIT },
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, Option<i64>>(6)?,
                    ))
                },
            ).map_err(sql_error)?
            .collect::<Result<Vec<_>, _>>().map_err(sql_error)?;
        let mut rows = Vec::with_capacity(found.len());
        for (id, phone, name, tier_id, points, credit, archived_at) in found {
            let tier_id = tier_id.map(|id| Id::parse(&id)).transpose()?;
            rows.push(CustomerRow {
                id: Id::parse(&id)?,
                phone,
                name,
                tier_name: running_tier_name(conn, tier_id.as_ref())?,
                points,
                credit: Money(credit),
                archived: archived_at.is_some(),
            });
        }
        Ok(rows)
    })
}

/// The customer page: details, balances, purchase history and points (CUS-03, CUS-04).
pub fn page(state: &AppState, id: &Id) -> Result<CustomerPage, AppError> {
    state.db.read(|conn| {
        let found = customers::get(conn, id)?;
        let purchases = purchases(conn, id)?;
        let mut total_spend = Money::zero();
        let mut visits: u32 = 0;
        for row in purchases.iter().filter(|row| !row.voided) {
            if row.kind == "sale" {
                total_spend = total_spend.checked_add(row.total)?;
                visits = visits.saturating_add(1);
            } else {
                total_spend = total_spend.checked_sub(row.total)?;
            }
        }
        Ok(CustomerPage {
            tier_name: running_tier_name(conn, found.customer.tier_id.as_ref())?,
            points_history: points_history(conn, id)?,
            customer: found.customer,
            points: found.points,
            credit: found.credit,
            oldest_unpaid_at: found.oldest_unpaid_at,
            total_spend,
            visits,
            purchases,
        })
    })
}

/// Adds a customer, or changes one (CUS-01). Without `can_manage` (quick create at the
/// till, CUS-02) the tier stays empty and the credit limit is the shop's default.
pub fn save(
    state: &AppState,
    by: &Id,
    can_manage: bool,
    input: &CustomerSave,
) -> Result<Id, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let country = shop_country(tx)?;
        let mut details = input.details.clone();
        if input.id.is_none() && !can_manage {
            let credit: CreditSettings = settings::get(tx)?;
            details.tier_id = None;
            details.credit_limit = credit.default_limit_minor;
        }
        if let Some(tier_id) = &details.tier_id {
            pricing::tier(tx, tier_id)?;
        }
        let (id, action) = match &input.id {
            Some(id) => {
                customers::update(tx, id, details.clone(), &country, clock.now())?;
                (id.clone(), CUSTOMER_CHANGE)
            }
            None => (
                customers::create(tx, details.clone(), &country, clock.now())?,
                CUSTOMER_CREATE,
            ),
        };
        let saved = customers::get(tx, &id)?.customer;
        let audit = serde_json::json!({
            "customer_id": id,
            "phone": saved.phone,
            "name": saved.name,
            "tier_id": saved.tier_id,
            "credit_limit_minor": saved.credit_limit.0,
            "loyalty_opt_out": saved.loyalty_opt_out,
        });
        audit_log::record(tx, clock, action, audit, Some(by.clone()))?;
        Ok(id)
    })
}

/// Archives a customer, or brings one back. Archived customers can't be found or
/// attached at the till; their ledgers stay.
pub fn set_archived(state: &AppState, by: &Id, id: &Id, archived: bool) -> Result<(), AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        customers::set_archived(tx, id, archived, clock.now())?;
        let details = serde_json::json!({ "customer_id": id, "archived": archived });
        audit_log::record(tx, clock, CUSTOMER_CHANGE, details, Some(by.clone()))
    })
}

/// The credit statement for a date range, both days included (CUS-05).
pub fn statement(
    state: &AppState,
    id: &Id,
    from: BusinessDate,
    to: BusinessDate,
) -> Result<Statement, AppError> {
    state
        .db
        .read(|conn| Ok(customers::statement(conn, id, from, to)?))
}

/// A payment against the customer's debt (CUS-05). Cash needs the user's open shift and
/// goes into its drawer; other methods may be taken in the back office too, and go on
/// the user's shift when they have one. A payment above the debt is refused
/// (`payment_more_than_debt`).
pub fn take_payment(
    state: &AppState,
    by: &Id,
    input: &CustomerPaymentInput,
) -> Result<CustomerPaymentDone, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        if input.amount.0 <= 0 {
            return Err(AppError::Validation {
                field: Some("amount".into()),
                code: "amount_not_positive".into(),
                params: Default::default(),
            });
        }
        let method = sales::payment_method(tx, &input.method_id)?;
        if !method.active {
            return Err(AppError::validation("payment_method_inactive"));
        }
        if !crate::workflows::checkout::takes_payment_now(method.kind) {
            return Err(AppError::validation("payment_method_not_for_debt"));
        }
        let reference = input
            .reference
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_string);
        if method.requires_reference && reference.is_none() {
            return Err(AppError::validation("payment_reference_missing"));
        }
        let shift = till_shift(tx)?.filter(|shift| shift.user_id == *by);
        let is_cash = method.kind == sales::PaymentKind::Cash;
        if is_cash && shift.is_none() {
            return Err(conflict("shift_not_open"));
        }
        let payment_id = Id::new();
        let ctx = ledger_ctx(tx, clock, by)?;
        let ledger_method = customers::PaymentMethod {
            method_kind: method.kind.as_str().to_string(),
            reference: reference.clone(),
        };
        let ref_ = LedgerRef {
            ref_type: "customer_payment".into(),
            ref_id: payment_id.clone(),
        };
        let credit = customers::add_credit(
            tx,
            &input.customer_id,
            input.amount.checked_neg()?,
            CreditKind::Payment,
            Some(&ledger_method),
            Some(&ref_),
            &ctx,
        )?;
        if let Some(shift) = &shift {
            shifts::record(
                tx,
                &shift.id,
                ShiftMovement {
                    kind: MovementKind::CustomerPayment,
                    method_id: Some(method.id.clone()),
                    method_kind: method.kind.as_str().to_string(),
                    amount: input.amount,
                    ref_type: Some("customer_payment".into()),
                    ref_id: Some(payment_id.clone()),
                    reason: None,
                    at: clock.now(),
                    user_id: by.clone(),
                },
            )?;
        }
        let details = serde_json::json!({
            "customer_id": input.customer_id,
            "payment_id": payment_id,
            "amount_minor": input.amount.0,
            "method": method.name,
            "reference": reference,
            "shift_id": shift.as_ref().map(|shift| shift.id.clone()),
            "balance_after_minor": credit.0,
        });
        audit_log::record(tx, clock, CUSTOMER_PAYMENT, details, Some(by.clone()))?;
        Ok(CustomerPaymentDone {
            credit,
            cash_taken: is_cash,
        })
    })
}

/// An admin adds or takes away points, with a reason (CUS-04). Gives the new balance.
pub fn adjust_points(
    state: &AppState,
    by: &Id,
    customer_id: &Id,
    points: i64,
    reason: &str,
) -> Result<i64, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let ctx = ledger_ctx(tx, clock, by)?;
        let balance = customers::add_points(
            tx,
            customer_id,
            points,
            PointsKind::Adjust,
            None,
            Some(reason),
            &ctx,
        )?;
        let details = serde_json::json!({
            "customer_id": customer_id,
            "points": points,
            "reason": reason.trim(),
            "balance_after": balance,
        });
        audit_log::record(tx, clock, POINTS_ADJUST, details, Some(by.clone()))?;
        Ok(balance)
    })
}

/// Who changed a ledger and when, with the till's business date.
pub(crate) fn ledger_ctx(
    conn: &Connection,
    clock: &dyn Clock,
    user: &Id,
) -> Result<LedgerCtx, AppError> {
    let till: TillSettings = settings::get(conn)?;
    let now = clock.now();
    Ok(LedgerCtx {
        at: now,
        business_date: business_date(
            now,
            clock.local_offset_minutes(),
            till.business_day_cutoff_hour,
        )?,
        user_id: Some(user.clone()),
    })
}

/// The loyalty settings in the form `customers` takes.
pub(crate) fn loyalty_view(conn: &Connection) -> Result<LoyaltySettingsView, AppError> {
    let loyalty: LoyaltySettings = settings::get(conn)?;
    Ok(LoyaltySettingsView {
        enabled: loyalty.enabled,
        spend_unit: loyalty.spend_unit_minor,
        points_per_unit: loyalty.points_per_unit,
        value_per_point: loyalty.value_per_point_minor,
        min_redeem: loyalty.min_redeem,
    })
}

/// The shop's country for cleaning phone numbers: the first two letters of its ISO 4217
/// currency code, which are the ISO country code. Example: "BDT" → "BD", "INR" → "IN".
pub(crate) fn shop_country(conn: &Connection) -> Result<String, AppError> {
    let currency: CurrencyFormat = settings::get(conn)?;
    Ok(currency
        .code
        .chars()
        .take(2)
        .collect::<String>()
        .to_uppercase())
}

/// The customer's sales and returns, newest first, from `sales`' published views.
fn purchases(conn: &Connection, customer: &Id) -> Result<Vec<PurchaseRow>, AppError> {
    let mut statement = conn
        .prepare(
            "SELECT 'sale', id, invoice_no, completed_at, total_minor, status = 'voided'
         FROM sales_v_sales WHERE customer_id = :customer
         UNION ALL
         SELECT 'return', return_id, return_no, at, SUM(gross_minor), 0
         FROM sales_v_return_lines WHERE customer_id = :customer
         GROUP BY return_id
         ORDER BY 4 DESC",
        )
        .map_err(sql_error)?;
    let rows = statement
        .query_map(named_params! { ":customer": customer.as_str() }, |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, bool>(5)?,
            ))
        })
        .map_err(sql_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error)?;
    rows.into_iter()
        .map(|(kind, id, number, at, total, voided)| {
            Ok(PurchaseRow {
                kind,
                id: Id::parse(&id)?,
                number,
                at: Timestamp(at),
                total: Money(total),
                voided,
            })
        })
        .collect()
}

/// The customer's points rows, newest first, from `customers`' published view.
fn points_history(conn: &Connection, customer: &Id) -> Result<Vec<PointsRow>, AppError> {
    let mut statement = conn
        .prepare(
            "SELECT at, kind, points_delta, reason FROM customers_v_points_ledger
         WHERE customer_id = :customer ORDER BY at DESC, id DESC LIMIT :limit",
        )
        .map_err(sql_error)?;
    let rows = statement
        .query_map(
            named_params! { ":customer": customer.as_str(), ":limit": POINTS_ROWS },
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .map_err(sql_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error)?;
    rows.into_iter()
        .map(|(at, kind, points, reason)| {
            let kind: PointsKind = serde_json::from_value(serde_json::Value::String(kind))
                .map_err(|error| AppError::internal(format!("points kind: {error}")))?;
            Ok(PointsRow {
                at: Timestamp(at),
                kind,
                points,
                reason,
            })
        })
        .collect()
}

/// A database error while reading the published views. The details go to the log.
fn sql_error(error: rusqlite::Error) -> AppError {
    AppError::internal(format!("customer page: {error}"))
}
