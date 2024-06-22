use inquire::Confirm;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde::Serialize;

use crate::errors::KakeboError;
use crate::KakeboConfig;

use super::money_amount;
use super::ExpenseInfo;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SingleExpense {
    amount: Decimal,
    info: ExpenseInfo,
}

impl SingleExpense {
    pub fn new(config: &KakeboConfig) -> Result<Self, KakeboError> {
        let info = ExpenseInfo::new()?;
        let amount = money_amount(config, &config.user_name)?;
        if Confirm::new("Save this expense?").prompt()? {
            Ok(Self { info, amount })
        } else {
            Err(KakeboError::ExpenseCreationAborted)
        }
    }
}
