use std::{
    collections::{BTreeSet, BinaryHeap, HashMap},
    fmt::format,
};

use crate::{errors::KakeboError, ExpenseType, KakeboConfig};
use edit::edit;
use serde::{de::DeserializeOwned, Deserialize, Deserializer};

const AMOUNT_TEMPLATE: &str = "amount = 0.0";

const KIND_TEMPLATE: &str = "
[kind] 
# uncomment a category or create your own 
# category = \"ReplacementOrRepair\" 
# category = \"Groceries\" 
# category = \"Social\" 
# category = \"Hobby\" 
# category = \"Restaurant\" 
# category = \"Entertainment\" 

# optionally, you can add a description (to explain the event or reason for the expense)
# description = \"description\"";

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

#[derive(Debug, Deserialize)]
struct ExpenseKind {
    category: ExpenseCategory,
    description: Option<String>,
}

trait Expense: DeserializeOwned {}

#[derive(Debug, Deserialize)]
pub struct SingleExpense {
    amount: i32,
    kind: ExpenseKind,
}

impl Expense for SingleExpense {}

fn cents_to_int<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    let amount_with_cents: f32 = Deserialize::deserialize(deserializer)?;
    let int_amount: i32 = (amount_with_cents * 100.0) as i32;
    Ok(int_amount)
}

#[derive(Debug, Deserialize)]
// a group expense that distributes extra costs such as tips or delivery fees fairly
struct GroupExpense {
    people: Vec<String>,
    raw_amounts: Vec<i32>,
    #[serde(deserialize_with = "cents_to_int")]
    total_amount: i32,
    paid_amounts: Vec<Option<i32>>,
}

impl GroupExpense {
    fn raw_total(&self) -> i32 {
        self.raw_amounts.iter().sum()
    }

    fn total_amounts(&self) -> Vec<i32> {
        self.raw_amounts
            .iter()
            .map(|raw_amount| raw_amount * self.total_amount)
            .map(|amount_with_total| {
                let mut scaled_amount = amount_with_total / self.raw_total();
                if amount_with_total % self.raw_total() >= self.raw_total() / 2 {
                    // round up to not lose a few cents every time
                    scaled_amount += 1;
                }
                scaled_amount
            })
            .collect()
    }
}

#[derive(Debug)]
struct RecurringExpense {}

#[derive(Debug)]
pub struct ExpenseEditor<T> {
    expense_type: ExpenseType,
    config: KakeboConfig,
    records: Vec<T>,
}

impl ExpenseEditor<SingleExpense> {
    fn record_template(&self) -> String {
        format!("{AMOUNT_TEMPLATE}\n{KIND_TEMPLATE}")
    }
}

impl ExpenseEditor<GroupExpense> {
    fn record_template(&self) -> String {
        let people: BTreeSet<&str> = self
            .records
            .iter()
            .flat_map(|record| record.people.iter())
            .map(|string_ref| string_ref.as_str())
            .collect();
        let mut str = format!(
            "{AMOUNT_TEMPLATE}\n{KIND_TEMPLATE}
[raw_amounts]
{} = 0.0
",
            self.config.user_name
        );
        for person_name in people {
            str.push_str(person_name);
            str.push_str(" = 0.0\n")
        }
        str
    }
}

impl<T: Expense> ExpenseEditor<T> {
    pub fn new(expense_type: ExpenseType, config: KakeboConfig) -> Self {
        Self {
            expense_type,
            config,
            records: Vec::new(),
        }
    }

    pub fn create_record(&mut self) -> Result<T, KakeboError> {
        let initial_content = self.record_template();
        let record_string = edit(initial_content)?;
        let record: T = toml::from_str(&record_string)?;
        Ok(record)
    }
}
