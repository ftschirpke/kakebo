use std::fmt::Display;

use inquire::Confirm;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde::Serialize;

use crate::errors::KakeboError;
use crate::DisplayableExpense;
use crate::KakeboConfig;

use super::money_amount;
use super::ExpenseInfo;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SingleExpense {
    pub amount: Decimal,
    pub info: ExpenseInfo,
}

impl Display for SingleExpense {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({:8.2})", self.info, self.amount)
    }
}

impl DisplayableExpense for SingleExpense {
    fn name() -> &'static str {
        "single expense"
    }

    fn plural_name() -> &'static str {
        "single expenses"
    }
}

impl SingleExpense {
    pub fn new(config: &KakeboConfig) -> Result<Self, KakeboError> {
        let info = ExpenseInfo::new()?;
        let amount = money_amount(config, &config.user_name)?;

        let new_instance = Self { info, amount };
        new_instance.configured_display(config);

        if Confirm::new("Save this expense?").prompt()? {
            Ok(new_instance)
        } else {
            Err(KakeboError::ExpenseCreationAborted)
        }
    }
}
