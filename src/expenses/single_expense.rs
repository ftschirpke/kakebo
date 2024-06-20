use rust_decimal::Decimal;
use serde::Deserialize;

use crate::errors::KakeboError;
use crate::KakeboConfig;

use super::Expense;
use super::ExpenseKind;

#[derive(Debug, Deserialize)]
pub struct SingleExpense {
    amount: Decimal,
    kind: ExpenseKind,
}

impl Expense for SingleExpense {
    fn record_template(_: &[Self], _: &KakeboConfig) -> String {
        format!("amount = 0.00\n{}", ExpenseKind::toml_template())
    }

    fn try_create(content: String) -> Result<Self, KakeboError> {
        let record = toml::from_str(&content)?;
        Ok(record)
    }
}
