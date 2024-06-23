use inquire::Text;
use serde::Deserialize;
use serde::Serialize;

use crate::errors::KakeboError;
use crate::KakeboConfig;

use super::single_expense::SingleExpense;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Debt {
    pub expense: SingleExpense,
    pub person: String,
}

impl Debt {
    pub fn new(config: &KakeboConfig) -> Result<Self, KakeboError> {
        let person = Text::new("Who do you owe this money to?").prompt()?; // TODO: name
                                                                           // suggestions
        let expense = SingleExpense::new(config)?;
        Ok(Self { expense, person })
    }
}
