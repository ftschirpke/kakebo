use std::fmt::Display;

use serde::Deserialize;
use serde::Serialize;

use crate::errors::KakeboError;
use crate::DisplayableExpense;
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

impl DisplayableExpense for Debt {
    fn name() -> &'static str {
        "debt owed"
    }

    fn plural_name() -> &'static str {
        "debts owed"
    }
}

impl Debt {
    pub fn new(environment: &Environment) -> Result<Self, KakeboError> {
        let person = person("Who do you owe this money to?", &environment.people)?;
        let expense = SingleExpense::new(&environment.config)?;

        let new_instance = Self { expense, person };
        new_instance.configured_display(&environment.config);

        Ok(new_instance)
    }
}
