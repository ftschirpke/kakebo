use chrono::NaiveDate;
use inquire::{
    error::{CustomUserError, InquireResult},
    required, Confirm, CustomType, DateSelect, MultiSelect, Select, Text,
};
use rust_decimal::Decimal;
use serde::{de::DeserializeOwned, Deserialize, Deserializer};

use crate::{errors::KakeboError, KakeboConfig};

pub mod group_expense;
// pub mod recurring_expense;
pub mod single_expense;

pub fn money_amount(config: &KakeboConfig) -> InquireResult<Decimal> {
    CustomType::new("Amount:")
        .with_formatter(&|decimal: Decimal| format!("{}{:.2}", config.currency, decimal))
        .with_error_message("Please type a valid number")
        .with_help_message(&format!(
            "Type the amount in {} using a decimal point as a separator",
            config.currency
        ))
        .prompt()
}

pub fn confirm() -> InquireResult<bool> {
    let ans = Confirm::new("Question?")
        .with_default(false)
        .with_help_message("This data is stored for good reasons")
        .prompt()?;
    println!("Your answer: {ans}");
    Ok(ans)
}

#[derive(Debug)]
pub struct ExpenseKind {
    category: ExpenseCategory,
    description: Option<String>,
    date: NaiveDate,
}

impl ExpenseKind {
    pub fn new() -> Result<Self, KakeboError> {
        let date = DateSelect::new("Date:").prompt()?;
        let category_text = Text::new("Category:")
            .with_validator(required!("Date is required!"))
            .with_autocomplete(&ExpenseCategory::suggestor)
            .prompt()?;
        let category = ExpenseCategory::from(category_text);
        let description = Text::new("Description:").prompt()?;
        let description = (!description.is_empty()).then_some(description);
        Ok(Self {
            date,
            description,
            category,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
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
        ]
    }
    fn suggestor(input: &str) -> Result<Vec<String>, CustomUserError> {
        let input = input.to_lowercase();
        let suggestions = Self::options()
            .into_iter()
            .filter(|option| option.to_lowercase().contains(&input))
            .map(str::to_string);
        Ok(suggestions.collect())
    }
}

pub trait Expense: Sized {
    fn record_template(records: &[Self], config: &KakeboConfig) -> String;
}
