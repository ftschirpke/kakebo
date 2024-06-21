use inquire::CustomType;
use rust_decimal::Decimal;
use serde::Deserialize;

use crate::errors::KakeboError;
use crate::KakeboConfig;

use super::confirm;
use super::money_amount;
use super::ExpenseKind;

#[derive(Debug)]
pub struct SingleExpense {
    amount: Decimal,
    kind: ExpenseKind,
}

impl SingleExpense {
    pub fn new(config: &KakeboConfig) -> Result<Self, KakeboError> {
        let kind = ExpenseKind::new()?;
        let amount = money_amount(config)?;
        let confimation = confirm()?;
        Ok(Self { kind, amount })
    }
}
