use std::{fmt::Display, iter::once};

use chrono::{Local, NaiveDate, Weekday};
use inquire::{
    error::InquireResult, required, validator::Validation, CustomType, DateSelect, Select, Text,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{errors::KakeboError, Environment, KakeboConfig};

pub mod advancement;
pub mod debt;
pub mod group_expense;
pub mod recurring_expense;
pub mod single_expense;

pub fn money_amount(config: &KakeboConfig, name: &str) -> InquireResult<Decimal> {
    CustomType::new(&format!("Amount {name}:"))
        .with_validator(|&input: &Decimal| {
            if input > Decimal::ZERO {
                Ok(Validation::Valid)
            } else {
                Ok(Validation::Invalid(
                    "Amount must be positive (non-zero).".into(),
                ))
            }
        })
        .with_formatter(&|decimal: Decimal| format!("{:.2}{}", decimal, config.currency))
        .with_error_message("Please type a valid number")
        .with_help_message(&format!(
            "Type the amount in {} using a decimal point as a separator",
            config.currency
        ))
        .prompt()
}

const NEW_PERSON: &str = "Add new Person";

pub fn person(prompt: &str, environment: &Environment) -> InquireResult<String> {
    let options_vec: Vec<_> = once(NEW_PERSON)
        .chain(environment.people.iter().map(String::as_str))
        .collect();
    let selected = Select::new(prompt, options_vec).prompt()?;
    if selected == NEW_PERSON {
        Text::new(prompt).prompt()
    } else {
        Ok(selected.to_string())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExpenseInfo {
    pub category: ExpenseCategory,
    pub description: Option<String>,
    pub date: NaiveDate,
    creation_date: NaiveDate,
}

impl ExpenseInfo {
    pub fn new() -> Result<Self, KakeboError> {
        let creation_date = Local::now().date_naive();
        let date = DateSelect::new("Date:")
            .with_week_start(Weekday::Mon)
            .prompt()?;
        let category_text = Select::new("Category:", ExpenseCategory::options()).prompt()?;
        let category_text = if category_text == "Other" {
            Text::new("Other category:")
                .with_validator(required!("Require non-empty category"))
                .prompt()?
        } else {
            category_text.to_string()
        };
        let category = ExpenseCategory::from(category_text);
        let description = Text::new("Description:").prompt()?;
        let description = (!description.is_empty()).then_some(description);
        Ok(Self {
            creation_date,
            date,
            description,
            category,
        })
    }
}

impl Display for ExpenseInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}: {}",
            self.date,
            self.category,
            self.description
                .as_ref()
                .map_or("No description", |descr| descr.as_str())
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExpenseCategory {
    ReplacementOrRepair,
    Groceries,
    Family,
    Friends,
    Hobby,
    Restaurant,
    Entertainment,
    Other(String),
}

impl From<String> for ExpenseCategory {
    fn from(value: String) -> Self {
        match value.as_str() {
            "Replacement or Repair" => Self::ReplacementOrRepair,
            "Groceries" => Self::Groceries,
            "Family" => Self::Family,
            "Friends" => Self::Friends,
            "Hobby" => Self::Hobby,
            "Restaurant" => Self::Restaurant,
            "Entertainment" => Self::Entertainment,
            _ => Self::Other(value),
        }
    }
}

impl Display for ExpenseCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            ExpenseCategory::ReplacementOrRepair => "Replacement or Repair",
            ExpenseCategory::Groceries => "Groceries",
            ExpenseCategory::Family => "Family",
            ExpenseCategory::Friends => "Friends",
            ExpenseCategory::Hobby => "Hobby",
            ExpenseCategory::Restaurant => "Restaurant",
            ExpenseCategory::Entertainment => "Entertainment",
            ExpenseCategory::Other(ref inner) => inner.as_str(),
        };
        write!(f, "{}", str)
    }
}

impl ExpenseCategory {
    fn options() -> Vec<&'static str> {
        vec![
            "Replacement or Repair",
            "Groceries",
            "Family",
            "Friends",
            "Hobby",
            "Restaurant",
            "Entertainment",
            "Other",
        ]
    }
}
