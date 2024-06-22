use serde::Deserialize;

use crate::KakeboConfig;

use super::Expense;

#[derive(Debug, Deserialize)]
pub struct RecurringExpense {}

impl Expense for RecurringExpense {
    fn record_template(records: &[Self], config: &KakeboConfig) -> String {
        todo!()
    }
    fn try_create(content: String) -> Result<Self, crate::errors::KakeboError> {
        todo!()
    }
}
