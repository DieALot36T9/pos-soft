//! Feature settings for checkout, stock, loyalty and credit (rule SET-17).

use pos_core::{AppError, Money};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Setting;
use crate::setting::invalid;

/// How the checkout screen behaves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct CheckoutSettings {
    /// Scanning the same item again adds to its line instead of a new line.
    pub merge_scans: bool,
    /// Lock the till after this many idle minutes, 1 to 120.
    pub idle_lock_min: u32,
    /// Count the cash before seeing the expected amount at shift close.
    pub blind_close: bool,
    /// Beeps on a good scan and on errors.
    pub sounds: bool,
}

impl Default for CheckoutSettings {
    fn default() -> CheckoutSettings {
        CheckoutSettings {
            merge_scans: true,
            idle_lock_min: 5,
            blind_close: true,
            sounds: true,
        }
    }
}

impl Setting for CheckoutSettings {
    const KEY: &'static str = "checkout";

    fn validate(&self) -> Result<(), AppError> {
        if !(1..=120).contains(&self.idle_lock_min) {
            return Err(invalid("idle_lock_min", "idle_lock_out_of_range"));
        }
        Ok(())
    }
}

/// What happens when a cashier sells from an expired batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ExpiredPolicy {
    Warn,
    Block,
}

/// How a product's cost changes when goods are received (ADR-018).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum CostMethod {
    MovingAverage,
    LastCost,
}

/// How stock is handled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct InventorySettings {
    /// Allow selling more than the stock on hand.
    pub allow_negative: bool,
    /// Batches expiring within this many days count as "near expiry", 0 to 365.
    pub near_expiry_days: u32,
    pub expired_policy: ExpiredPolicy,
    pub cost_method: CostMethod,
}

impl Default for InventorySettings {
    fn default() -> InventorySettings {
        InventorySettings {
            allow_negative: true,
            near_expiry_days: 30,
            expired_policy: ExpiredPolicy::Warn,
            cost_method: CostMethod::MovingAverage,
        }
    }
}

impl Setting for InventorySettings {
    const KEY: &'static str = "inventory";

    fn validate(&self) -> Result<(), AppError> {
        if self.near_expiry_days > 365 {
            return Err(invalid("near_expiry_days", "near_expiry_days_out_of_range"));
        }
        Ok(())
    }
}

/// Loyalty points. Example with the defaults: every 100.00 spent earns 1 point,
/// and 1 point is worth 1.00 off.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct LoyaltySettings {
    pub enabled: bool,
    /// The spend that earns points. Example: Money(10000) is 100.00.
    pub spend_unit_minor: Money,
    /// Points earned for each full spend unit.
    pub points_per_unit: i64,
    /// What one point is worth when paying.
    pub value_per_point_minor: Money,
    /// The fewest points a customer may pay with.
    pub min_redeem: i64,
}

impl Default for LoyaltySettings {
    fn default() -> LoyaltySettings {
        LoyaltySettings {
            enabled: false,
            spend_unit_minor: Money(10_000),
            points_per_unit: 1,
            value_per_point_minor: Money(100),
            min_redeem: 0,
        }
    }
}

impl Setting for LoyaltySettings {
    const KEY: &'static str = "loyalty";

    fn validate(&self) -> Result<(), AppError> {
        if self.spend_unit_minor.0 <= 0 {
            return Err(invalid(
                "spend_unit_minor",
                "loyalty_spend_unit_not_positive",
            ));
        }
        if self.points_per_unit <= 0 {
            return Err(invalid("points_per_unit", "loyalty_points_not_positive"));
        }
        if self.value_per_point_minor.0 <= 0 {
            return Err(invalid(
                "value_per_point_minor",
                "loyalty_point_value_not_positive",
            ));
        }
        if self.min_redeem < 0 {
            return Err(invalid("min_redeem", "loyalty_min_redeem_negative"));
        }
        Ok(())
    }
}

/// Selling on credit (baki).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct CreditSettings {
    pub enabled: bool,
    /// The limit new customers get. 0 means no credit until an admin sets one.
    pub default_limit_minor: Money,
}

impl Default for CreditSettings {
    fn default() -> CreditSettings {
        CreditSettings {
            enabled: false,
            default_limit_minor: Money::zero(),
        }
    }
}

impl Setting for CreditSettings {
    const KEY: &'static str = "credit";

    fn validate(&self) -> Result<(), AppError> {
        if self.default_limit_minor.0 < 0 {
            return Err(invalid("default_limit_minor", "credit_limit_negative"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code_of(result: Result<(), AppError>) -> String {
        match result {
            Err(AppError::Validation { code, .. }) => code,
            other => format!("{other:?}"),
        }
    }

    #[test]
    fn set_17_defaults_match_the_requirement() {
        let inventory = InventorySettings::default();
        assert!(inventory.allow_negative);
        assert_eq!(inventory.near_expiry_days, 30);
        let checkout = CheckoutSettings::default();
        assert!(checkout.merge_scans);
        assert_eq!(checkout.idle_lock_min, 5);
        assert!(checkout.blind_close);
        for result in [
            inventory.validate(),
            checkout.validate(),
            LoyaltySettings::default().validate(),
            CreditSettings::default().validate(),
        ] {
            assert!(result.is_ok());
        }
    }

    #[test]
    fn set_17_idle_lock_is_1_to_120_minutes() {
        let never = CheckoutSettings {
            idle_lock_min: 0,
            ..CheckoutSettings::default()
        };
        assert_eq!(code_of(never.validate()), "idle_lock_out_of_range");
    }

    #[test]
    fn set_17_near_expiry_is_at_most_a_year() {
        let too_long = InventorySettings {
            near_expiry_days: 366,
            ..InventorySettings::default()
        };
        assert_eq!(
            code_of(too_long.validate()),
            "near_expiry_days_out_of_range"
        );
    }

    #[test]
    fn set_17_loyalty_rules() {
        let cases = [
            (
                LoyaltySettings {
                    spend_unit_minor: Money(0),
                    ..LoyaltySettings::default()
                },
                "loyalty_spend_unit_not_positive",
            ),
            (
                LoyaltySettings {
                    points_per_unit: 0,
                    ..LoyaltySettings::default()
                },
                "loyalty_points_not_positive",
            ),
            (
                LoyaltySettings {
                    value_per_point_minor: Money(0),
                    ..LoyaltySettings::default()
                },
                "loyalty_point_value_not_positive",
            ),
            (
                LoyaltySettings {
                    min_redeem: -1,
                    ..LoyaltySettings::default()
                },
                "loyalty_min_redeem_negative",
            ),
        ];
        for (loyalty, expected) in cases {
            assert_eq!(code_of(loyalty.validate()), expected);
        }
    }

    #[test]
    fn set_17_credit_limit_cannot_be_negative() {
        let credit = CreditSettings {
            enabled: true,
            default_limit_minor: Money(-1),
        };
        assert_eq!(code_of(credit.validate()), "credit_limit_negative");
    }
}
