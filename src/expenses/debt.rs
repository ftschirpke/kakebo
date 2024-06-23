use std::fmt::Display;

use serde::Deserialize;
use serde::Serialize;

use crate::errors::KakeboError;
use crate::Environment;

use super::person;
use super::single_expense::SingleExpense;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Debt {
    pub expense: SingleExpense,
    pub person: String,
}

impl Display for Debt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({:8.2} owed to {})",
            self.expense.info, self.expense.amount, self.person
        )
    }
}

impl Debt {
    pub fn new(environment: &Environment) -> Result<Self, KakeboError> {
        let person = person("Who do you owe this money to?", environment)?;
        let expense = SingleExpense::new(&environment.config)?;
        Ok(Self { expense, person })
    }
}
