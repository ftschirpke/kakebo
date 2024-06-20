use serde::{de::DeserializeOwned, Deserialize, Deserializer};

use crate::{errors::KakeboError, KakeboConfig};

pub mod group_expense;
pub mod recurring_expense;
pub mod single_expense;

#[derive(Debug, Deserialize)]
pub struct ExpenseKind {
    category: ExpenseCategory,
    description: Option<String>,
}

impl ExpenseKind {
    pub fn toml_template() -> &'static str {
        "
[kind] 
# uncomment a category or create your own 
# category = \"ReplacementOrRepair\" 
# category = \"Groceries\" 
# category = \"Social\" 
# category = \"Hobby\" 
# category = \"Restaurant\" 
# category = \"Entertainment\" 

# optionally, you can add a description (to explain the event or reason for the expense)
# description = \"description\""
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

pub trait Expense: DeserializeOwned {
    fn record_template(records: &[Self], config: &KakeboConfig) -> String;
    fn try_create(content: String) -> Result<Self, KakeboError>;
}
