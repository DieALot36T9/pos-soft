//! Saving and reading promotions and their targets (PRC-04).

use pos_core::{AppError, Bps, Id, Money, Timestamp};
use rusqlite::{Connection, OptionalExtension, Transaction, named_params};

use crate::error::{PricingError, invalid, not_found, stored_id};
use crate::tiers::tier;
use crate::{EVERY_DAY, PromoRule, PromoTarget, Promotion, PromotionInput};

const SELECT_PROMOTIONS: &str = "SELECT id, name, kind, percent_bps, amount_minor, buy_qty,
    get_qty, bundle_qty, starts_at, ends_at, days_mask, tier_id, active, created_at
    FROM pricing_promotions";

/// Makes a new, switched-on promotion.
pub fn create_promotion(
    tx: &Transaction,
    input: &PromotionInput,
    now: Timestamp,
) -> Result<Id, PricingError> {
    check_input(tx, input)?;
    let id = Id::new();
    // Start the row with what never changes; `write_settings` fills in the rest.
    tx.execute(
        "INSERT INTO pricing_promotions (id, name, kind, active, created_at)
         VALUES (:id, '', :kind, 1, :created_at)",
        named_params! { ":id": id.as_str(), ":kind": input.rule.kind(), ":created_at": now.0 },
    )?;
    write_settings(tx, &id, input)?;
    Ok(id)
}

/// Replaces a promotion's settings and targets. Its switch and creation time stay.
pub fn update_promotion(
    tx: &Transaction,
    id: &Id,
    input: &PromotionInput,
) -> Result<(), PricingError> {
    require_promotion(tx, id)?;
    check_input(tx, input)?;
    write_settings(tx, id, input)
}

/// Switches a promotion on or off (the "active switch" in PRC-04).
pub fn set_promotion_active(tx: &Transaction, id: &Id, active: bool) -> Result<(), PricingError> {
    require_promotion(tx, id)?;
    tx.execute(
        "UPDATE pricing_promotions SET active = :active WHERE id = :id",
        named_params! { ":id": id.as_str(), ":active": active },
    )?;
    Ok(())
}

/// One promotion with its targets.
pub fn promotion(conn: &Connection, id: &Id) -> Result<Promotion, PricingError> {
    let sql = format!("{SELECT_PROMOTIONS} WHERE id = :id");
    let row = conn
        .query_row(&sql, named_params! { ":id": id.as_str() }, read_row)
        .optional()?;
    match row {
        Some(row) => to_promotion(conn, row),
        None => Err(not_found("promotion", id)),
    }
}

/// Every promotion, oldest first.
pub fn promotions(conn: &Connection) -> Result<Vec<Promotion>, PricingError> {
    let sql = format!("{SELECT_PROMOTIONS} ORDER BY created_at, id");
    read_many(conn, &sql, [])
}

/// Switched-on promotions that haven't ended at `now`, oldest first. The engine checks
/// the rest (start, weekday, tier) itself.
pub(crate) fn not_ended(conn: &Connection, now: Timestamp) -> Result<Vec<Promotion>, PricingError> {
    let sql = format!(
        "{SELECT_PROMOTIONS} WHERE active = 1 AND (ends_at IS NULL OR ends_at > :now)
         ORDER BY created_at, id"
    );
    read_many(conn, &sql, named_params! { ":now": now.0 })
}

fn read_many(
    conn: &Connection,
    sql: &str,
    params: impl rusqlite::Params,
) -> Result<Vec<Promotion>, PricingError> {
    let mut statement = conn.prepare(sql)?;
    let rows = statement.query_map(params, read_row)?;
    let mut list = Vec::new();
    for row in rows {
        list.push(to_promotion(conn, row?)?);
    }
    Ok(list)
}

/// Writes everything except the switch and creation time, and replaces the targets.
fn write_settings(tx: &Transaction, id: &Id, input: &PromotionInput) -> Result<(), PricingError> {
    let (percent, amount, buy, get, bundle) = rule_columns(&input.rule);
    tx.execute(
        "UPDATE pricing_promotions SET name = :name, kind = :kind, percent_bps = :percent,
           amount_minor = :amount, buy_qty = :buy, get_qty = :get, bundle_qty = :bundle,
           starts_at = :starts, ends_at = :ends, days_mask = :days, tier_id = :tier
         WHERE id = :id",
        named_params! {
            ":id": id.as_str(), ":name": input.name.trim(), ":kind": input.rule.kind(),
            ":percent": percent, ":amount": amount, ":buy": buy, ":get": get, ":bundle": bundle,
            ":starts": input.starts_at.map(|at| at.0), ":ends": input.ends_at.map(|at| at.0),
            ":days": input.days_mask, ":tier": input.tier_id.as_ref().map(Id::as_str),
        },
    )?;
    tx.execute(
        "DELETE FROM pricing_promotion_targets WHERE promotion_id = :id",
        named_params! { ":id": id.as_str() },
    )?;
    let mut targets = input.targets.clone();
    targets.sort();
    targets.dedup();
    let mut insert = tx.prepare_cached(
        "INSERT INTO pricing_promotion_targets (promotion_id, target_type, target_id)
         VALUES (:id, :type, :target)",
    )?;
    for target in &targets {
        insert.execute(named_params! {
            ":id": id.as_str(), ":type": target.type_name(), ":target": target.id().as_str(),
        })?;
    }
    Ok(())
}

/// The rule's numbers as (percent_bps, amount_minor, buy_qty, get_qty, bundle_qty).
type RuleColumns = (
    Option<u32>,
    Option<i64>,
    Option<u32>,
    Option<u32>,
    Option<u32>,
);

fn rule_columns(rule: &PromoRule) -> RuleColumns {
    match rule {
        PromoRule::PercentOff { percent } => (Some(percent.0), None, None, None, None),
        PromoRule::AmountOff { amount_per_unit } => {
            (None, Some(amount_per_unit.0), None, None, None)
        }
        PromoRule::FixedPrice { price_per_unit } => {
            (None, Some(price_per_unit.0), None, None, None)
        }
        PromoRule::BuyXGetY { buy, get } => (None, None, Some(*buy), Some(*get), None),
        PromoRule::MultiBuy {
            bundle_qty,
            bundle_price,
        } => (None, Some(bundle_price.0), None, None, Some(*bundle_qty)),
    }
}

/// Builds the rule back from its columns. Missing numbers mean the row is broken: a bug.
fn rule_from_columns(kind: &str, columns: RuleColumns) -> Result<PromoRule, PricingError> {
    let broken = || {
        PricingError::App(AppError::internal(format!(
            "promotion row is broken: {kind}"
        )))
    };
    let rule = match (kind, columns) {
        ("percent_off", (Some(percent), _, _, _, _)) => PromoRule::PercentOff {
            percent: Bps(percent),
        },
        ("amount_off", (_, Some(amount), _, _, _)) => PromoRule::AmountOff {
            amount_per_unit: Money(amount),
        },
        ("fixed_price", (_, Some(price), _, _, _)) => PromoRule::FixedPrice {
            price_per_unit: Money(price),
        },
        ("buy_x_get_y", (_, _, Some(buy), Some(get), _)) => PromoRule::BuyXGetY { buy, get },
        ("multi_buy", (_, Some(price), _, _, Some(bundle_qty))) => PromoRule::MultiBuy {
            bundle_qty,
            bundle_price: Money(price),
        },
        _ => return Err(broken()),
    };
    Ok(rule)
}

/// Checks a promotion before saving it. Each code tells the UI which message to show.
fn check_input(conn: &Connection, input: &PromotionInput) -> Result<(), PricingError> {
    if input.name.trim().is_empty() {
        return Err(invalid("name", "promotion_name_missing"));
    }
    match &input.rule {
        PromoRule::PercentOff { percent } if percent.0 == 0 || percent.0 > 10_000 => {
            return Err(invalid("percent", "promotion_percent_invalid"));
        }
        PromoRule::AmountOff { amount_per_unit } if amount_per_unit.0 <= 0 => {
            return Err(invalid("amount", "promotion_amount_invalid"));
        }
        PromoRule::FixedPrice { price_per_unit } if price_per_unit.0 < 0 => {
            return Err(invalid("price", "promotion_price_invalid"));
        }
        PromoRule::BuyXGetY { buy, get } if *buy == 0 || *get == 0 => {
            return Err(invalid("buy_qty", "promotion_buy_get_invalid"));
        }
        PromoRule::MultiBuy {
            bundle_qty,
            bundle_price,
        } if *bundle_qty < 2 || bundle_price.0 < 0 => {
            return Err(invalid("bundle_qty", "promotion_bundle_invalid"));
        }
        _ => {}
    }
    if input.days_mask == 0 || input.days_mask > EVERY_DAY {
        return Err(invalid("days", "promotion_days_invalid"));
    }
    if let (Some(starts), Some(ends)) = (input.starts_at, input.ends_at)
        && starts >= ends
    {
        return Err(invalid("ends_at", "promotion_dates_invalid"));
    }
    if input.targets.is_empty() {
        return Err(invalid("targets", "promotion_targets_missing"));
    }
    if let Some(tier_id) = &input.tier_id {
        tier(conn, tier_id)?;
    }
    Ok(())
}

fn require_promotion(conn: &Connection, id: &Id) -> Result<(), PricingError> {
    let found: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM pricing_promotions WHERE id = :id",
            named_params! { ":id": id.as_str() },
            |row| row.get(0),
        )
        .optional()?;
    match found {
        Some(_) => Ok(()),
        None => Err(not_found("promotion", id)),
    }
}

/// One `pricing_promotions` row, as stored.
struct PromotionRow {
    id: String,
    name: String,
    kind: String,
    columns: RuleColumns,
    starts_at: Option<i64>,
    ends_at: Option<i64>,
    days_mask: u8,
    tier_id: Option<String>,
    active: bool,
    created_at: i64,
}

fn read_row(row: &rusqlite::Row) -> rusqlite::Result<PromotionRow> {
    Ok(PromotionRow {
        id: row.get(0)?,
        name: row.get(1)?,
        kind: row.get(2)?,
        columns: (
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
            row.get(6)?,
            row.get(7)?,
        ),
        starts_at: row.get(8)?,
        ends_at: row.get(9)?,
        days_mask: row.get(10)?,
        tier_id: row.get(11)?,
        active: row.get(12)?,
        created_at: row.get(13)?,
    })
}

fn to_promotion(conn: &Connection, row: PromotionRow) -> Result<Promotion, PricingError> {
    let id = stored_id(&row.id)?;
    let tier_id = match row.tier_id {
        Some(text) => Some(stored_id(&text)?),
        None => None,
    };
    Ok(Promotion {
        rule: rule_from_columns(&row.kind, row.columns)?,
        targets: targets_of(conn, &id)?,
        id,
        name: row.name,
        starts_at: row.starts_at.map(Timestamp),
        ends_at: row.ends_at.map(Timestamp),
        days_mask: row.days_mask,
        tier_id,
        active: row.active,
        created_at: Timestamp(row.created_at),
    })
}

/// A promotion's targets, sorted.
fn targets_of(conn: &Connection, id: &Id) -> Result<Vec<PromoTarget>, PricingError> {
    let mut statement = conn.prepare_cached(
        "SELECT target_type, target_id FROM pricing_promotion_targets WHERE promotion_id = :id",
    )?;
    let rows = statement.query_map(named_params! { ":id": id.as_str() }, |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut targets = Vec::new();
    for row in rows {
        let (target_type, target_id) = row?;
        let target_id = stored_id(&target_id)?;
        let target = match target_type.as_str() {
            "product" => PromoTarget::Product(target_id),
            "category" => PromoTarget::Category(target_id),
            "brand" => PromoTarget::Brand(target_id),
            other => {
                let detail = format!("promotion target type is unknown: {other}");
                return Err(PricingError::App(AppError::internal(detail)));
            }
        };
        targets.push(target);
    }
    targets.sort();
    Ok(targets)
}
