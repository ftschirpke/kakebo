use std::fmt::Display;

use chrono::{Local, NaiveDate, Weekday};
use inquire::{DateSelect, Text};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{errors::KakeboError, DisplayableExpense, Environment};

use super::{money_amount, person};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Advancement {
    pub person: String,
    pub amount: Decimal,
    description: Option<String>,
    date: NaiveDate,
    creation_date: NaiveDate,
}

impl Display for Advancement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({:8.2} owed by {})",
            self.description
                .as_ref()
                .map_or("No description", |descr| descr.as_str()),
            self.amount,
            self.person
        )
    }
}

impl DisplayableExpense for Advancement {
    fn name() -> &'static str {
        "unpaid advancement"
    }

    fn plural_name() -> &'static str {
        "unpaid advancements"
    }
}

impl Advancement {
    pub fn new(environment: &Environment) -> Result<Self, KakeboError> {
        let creation_date = Local::now().date_naive();
        let person = person("Who owes you this money?", &environment.people)?;
        let date = DateSelect::new("Date:")
            .with_week_start(Weekday::Mon)
            .prompt()?;
        let description = Text::new("Description:").prompt()?;
        let description = (!description.is_empty()).then_some(description);
        let amount = money_amount(&environment.config, &person)?;

        let new_instance = Self {
            person,
            amount,
            creation_date,
            date,
            description,
        };
        new_instance.configured_display(&environment.config);

        Ok(new_instance)
    }
}
