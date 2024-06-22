use inquire::Confirm;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde::Serialize;

use crate::errors::KakeboError;
use crate::KakeboConfig;

use super::money_amount;
use super::ExpenseKind;

#[derive(Debug, Serialize, Deserialize)]
pub struct SingleExpense {
    amount: Decimal,
    kind: ExpenseKind,
}

impl SingleExpense {
    pub fn new(config: &KakeboConfig) -> Result<Self, KakeboError> {
        let kind = ExpenseKind::new()?;
        let amount = money_amount(config, &config.user_name)?;
        if Confirm::new("Save this expense?").prompt()? {
            Ok(Self { kind, amount })
        } else {
            Err(KakeboError::ExpenseCreationAborted)
        }
    }
}
