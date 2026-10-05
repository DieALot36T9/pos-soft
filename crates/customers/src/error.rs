//! What can go wrong in the customers module, and how it reaches the UI.

use std::collections::BTreeMap;

use pos_core::{AppError, Money};

/// Everything that can go wrong in the customers module.
/// `app` turns it into an `AppError` with `?`; the UI only sees the codes below.
#[derive(Debug, thiserror::Error)]
pub enum CustomersError {
    /// The phone number can't be a real number. Code `phone_invalid`.
    #[error("the phone number is not valid")]
    PhoneInvalid,
    /// The shop's country code isn't a known country. Code `country_invalid`.
    #[error("the country {0} is not known")]
    CountryInvalid(String),
    /// Every customer needs a name. Code `customer_name_missing`.
    #[error("the customer has no name")]
    NameMissing,
    /// A credit limit below zero makes no sense. Code `credit_limit_negative`.
    #[error("the credit limit is below zero")]
    CreditLimitNegative,
    /// Another customer already has this phone number. Code `phone_in_use`.
    #[error("the phone number is already used by {name}")]
    PhoneInUse { name: String },
    /// No customer has this id.
    #[error("customer {0} was not found")]
    CustomerNotFound(String),
    /// A ledger row must change something. Code `amount_zero`.
    #[error("the amount is zero")]
    AmountZero,
    /// The amount's sign doesn't fit the kind of row, e.g. a negative credit sale.
    /// Code `amount_wrong_sign`.
    #[error("a {kind} row can't have this sign")]
    AmountWrongSign { kind: &'static str },
    /// Admin adjustments must say why (rule CUS-04). Code `reason_missing`.
    #[error("an adjustment needs a reason")]
    ReasonMissing,
    /// The customer doesn't have enough points. Code `points_not_enough`.
    #[error("the customer has only {balance} points")]
    PointsNotEnough { balance: i64 },
    /// The customer chose not to collect points. Code `loyalty_opted_out`.
    #[error("the customer does not collect points")]
    LoyaltyOptedOut,
    /// More store credit was used than the shop owes. Code `store_credit_not_enough`.
    #[error("only {available:?} of store credit is left")]
    StoreCreditNotEnough { available: Money },
    /// A payment can't be more than the customer owes. Code `payment_more_than_debt`.
    #[error("the customer owes only {owed:?}")]
    PaymentMoreThanDebt { owed: Money },
    /// This document was already reversed (voided). Code `already_reversed`.
    #[error("{ref_type} {ref_id} was already reversed")]
    AlreadyReversed { ref_type: String, ref_id: String },
    /// The statement's start date is after its end date. Code `date_range_invalid`.
    #[error("the start date is after the end date")]
    DateRangeInvalid,
    /// An error from `core`, such as a money overflow.
    #[error(transparent)]
    App(#[from] AppError),
    /// SQLite refused something. The details go to the log.
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

impl From<CustomersError> for AppError {
    fn from(error: CustomersError) -> AppError {
        match error {
            CustomersError::PhoneInvalid => validation(Some("phone"), "phone_invalid", &[]),
            CustomersError::CountryInvalid(_) => validation(None, "country_invalid", &[]),
            CustomersError::NameMissing => validation(Some("name"), "customer_name_missing", &[]),
            CustomersError::CreditLimitNegative => {
                validation(Some("credit_limit"), "credit_limit_negative", &[])
            }
            CustomersError::AmountZero => validation(Some("amount"), "amount_zero", &[]),
            CustomersError::AmountWrongSign { .. } => {
                validation(Some("amount"), "amount_wrong_sign", &[])
            }
            CustomersError::ReasonMissing => validation(Some("reason"), "reason_missing", &[]),
            CustomersError::DateRangeInvalid => validation(None, "date_range_invalid", &[]),
            CustomersError::PhoneInUse { name } => conflict("phone_in_use", &[("name", name)]),
            CustomersError::PointsNotEnough { balance } => {
                conflict("points_not_enough", &[("balance", balance.to_string())])
            }
            CustomersError::LoyaltyOptedOut => conflict("loyalty_opted_out", &[]),
            CustomersError::StoreCreditNotEnough { available } => conflict(
                "store_credit_not_enough",
                &[("available_minor", available.0.to_string())],
            ),
            CustomersError::PaymentMoreThanDebt { owed } => conflict(
                "payment_more_than_debt",
                &[("owed_minor", owed.0.to_string())],
            ),
            CustomersError::AlreadyReversed { .. } => conflict("already_reversed", &[]),
            CustomersError::CustomerNotFound(key) => AppError::NotFound {
                entity: "customer".to_string(),
                key,
            },
            CustomersError::App(inner) => inner,
            CustomersError::Sqlite(inner) => AppError::internal(format!("database error: {inner}")),
        }
    }
}

fn validation(field: Option<&str>, code: &str, params: &[(&str, String)]) -> AppError {
    AppError::Validation {
        field: field.map(str::to_string),
        code: code.to_string(),
        params: to_map(params),
    }
}

fn conflict(code: &str, params: &[(&str, String)]) -> AppError {
    AppError::Conflict {
        code: code.to_string(),
        params: to_map(params),
    }
}

fn to_map(params: &[(&str, String)]) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for (key, value) in params {
        map.insert((*key).to_string(), value.clone());
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phone_in_use_names_the_other_customer() {
        let error = AppError::from(CustomersError::PhoneInUse {
            name: "Rahim".to_string(),
        });
        let json = serde_json::to_string(&error).unwrap();
        assert_eq!(
            json,
            r#"{"kind":"conflict","code":"phone_in_use","params":{"name":"Rahim"}}"#
        );
    }

    #[test]
    fn database_details_never_reach_the_ui() {
        let error = AppError::from(CustomersError::Sqlite(rusqlite::Error::InvalidColumnName(
            "secret_column".to_string(),
        )));
        let json = serde_json::to_string(&error).unwrap();
        assert!(!json.contains("secret_column"));
    }
}
