use chrono::{Local, NaiveDate};
use inquire::{error::InquireResult, required, CustomType, DateSelect, Select, Text};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{errors::KakeboError, KakeboConfig};

pub mod group_expense;
// pub mod recurring_expense;
pub mod single_expense;

pub fn money_amount(config: &KakeboConfig, name: &str) -> InquireResult<Decimal> {
    CustomType::new(&format!("Amount {name}:"))
        .with_formatter(&|decimal: Decimal| format!("{:.2}{}", decimal, config.currency))
        .with_error_message("Please type a valid number")
        .with_help_message(&format!(
            "Type the amount in {} using a decimal point as a separator",
            config.currency
        ))
        .prompt()
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExpenseInfo {
    category: ExpenseCategory,
    description: Option<String>,
    date: NaiveDate,
    creation_date: NaiveDate,
}

impl ExpenseInfo {
    pub fn new() -> Result<Self, KakeboError> {
        let creation_date = Local::now().date_naive();
        let date = DateSelect::new("Date:").prompt()?;
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
enum ExpenseCategory {
    ReplacementOrRepair,
    Groceries,
    Social,
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
            "Social" => Self::Social,
            "Hobby" => Self::Hobby,
            "Restaurant" => Self::Restaurant,
            "Entertainment" => Self::Entertainment,
            _ => Self::Other(value),
        }
    }
}

impl ExpenseCategory {
    fn options() -> Vec<&'static str> {
        vec![
            "Replacement or Repair",
            "Groceries",
            "Social",
            "Hobby",
            "Restaurant",
            "Entertainment",
            "Other",
        ]
    }
}
