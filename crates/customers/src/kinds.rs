//! How ledger kinds are written in the database, and which signs each kind allows.

use crate::{CreditKind, CustomersError, PointsKind};

impl PointsKind {
    /// The text stored in `customers_points_ledger.kind`.
    pub(crate) fn as_db(self) -> &'static str {
        match self {
            PointsKind::Earn => "earn",
            PointsKind::Redeem => "redeem",
            PointsKind::Reverse => "reverse",
            PointsKind::Adjust => "adjust",
        }
    }

    /// Checks the sign: earning adds points and redeeming takes them away.
    pub(crate) fn check_sign(self, delta: i64) -> Result<(), CustomersError> {
        let fits = match self {
            PointsKind::Earn => delta > 0,
            PointsKind::Redeem => delta < 0,
            PointsKind::Reverse | PointsKind::Adjust => true,
        };
        check(delta, fits, self.as_db())
    }
}

impl CreditKind {
    /// The text stored in `customers_credit_ledger.kind`.
    pub(crate) fn as_db(self) -> &'static str {
        match self {
            CreditKind::CreditSale => "credit_sale",
            CreditKind::Payment => "payment",
            CreditKind::Return => "return",
            CreditKind::StoreCredit => "store_credit",
            CreditKind::StoreCreditUsed => "store_credit_used",
            CreditKind::Adjust => "adjust",
            CreditKind::VoidReversal => "void_reversal",
        }
    }

    /// Reads a kind back from the database.
    pub(crate) fn from_db(text: &str) -> Result<CreditKind, CustomersError> {
        let kind = match text {
            "credit_sale" => CreditKind::CreditSale,
            "payment" => CreditKind::Payment,
            "return" => CreditKind::Return,
            "store_credit" => CreditKind::StoreCredit,
            "store_credit_used" => CreditKind::StoreCreditUsed,
            "adjust" => CreditKind::Adjust,
            "void_reversal" => CreditKind::VoidReversal,
            other => return Err(crate::rows::corrupt(format!("credit kind {other}"))),
        };
        Ok(kind)
    }

    /// Checks the sign: positive rows add to what the customer owes, negative rows take away.
    pub(crate) fn check_sign(self, amount: i64) -> Result<(), CustomersError> {
        let fits = match self {
            CreditKind::CreditSale | CreditKind::StoreCreditUsed => amount > 0,
            CreditKind::Payment | CreditKind::Return | CreditKind::StoreCredit => amount < 0,
            CreditKind::Adjust | CreditKind::VoidReversal => true,
        };
        check(amount, fits, self.as_db())
    }
}

fn check(amount: i64, sign_fits: bool, kind: &'static str) -> Result<(), CustomersError> {
    if amount == 0 {
        return Err(CustomersError::AmountZero);
    }
    if !sign_fits {
        return Err(CustomersError::AmountWrongSign { kind });
    }
    Ok(())
}
