use chrono::{Local, NaiveDate, Weekday};
use inquire::{DateSelect, Text};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{errors::KakeboError, KakeboConfig};

use super::money_amount;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Advancement {
    pub person: String,
    pub amount: Decimal,
    description: Option<String>,
    date: NaiveDate,
    creation_date: NaiveDate,
}
impl Advancement {
    pub fn new(config: &KakeboConfig) -> Result<Self, KakeboError> {
        let creation_date = Local::now().date_naive();
        let person = Text::new("Who owes you this money?").prompt()?; // TODO: name suggestion
        let date = DateSelect::new("Date:")
            .with_week_start(Weekday::Mon)
            .prompt()?;
        let description = Text::new("Description:").prompt()?;
        let description = (!description.is_empty()).then_some(description);
        let amount = money_amount(config, &person)?;
        Ok(Self {
            person,
            amount,
            creation_date,
            date,
            description,
        })
    }
}
