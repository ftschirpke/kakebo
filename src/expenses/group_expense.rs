use std::collections::{BTreeSet, HashMap};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{errors::KakeboError, KakeboConfig};

use super::{Expense, ExpenseKind};

#[derive(Debug, Deserialize, Serialize)]
struct GroupExpenseIntermediate {
    raw_amounts: HashMap<String, Decimal>,
    total_amount: Decimal,
    paid_amounts: HashMap<String, Decimal>,
}

#[derive(Debug, Deserialize)]
/// a group expense that distributes extra costs such as tips or delivery fees fairly
pub struct GroupExpense {
    people: Vec<String>,
    raw_amounts: Vec<Decimal>,
    total_amount: Decimal,
    paid_amounts: Vec<Option<Decimal>>,
}

impl From<GroupExpenseIntermediate> for GroupExpense {
    fn from(value: GroupExpenseIntermediate) -> Self {
        let (people, raw_amounts): (Vec<_>, Vec<_>) = value
            .raw_amounts
            .into_iter()
            .filter(|(_, amount)| !amount.is_zero())
            .unzip();
        let mut paid_map = value.paid_amounts;
        let paid_amounts: Vec<_> = people
            .iter()
            .map(|name| paid_map.remove(name))
            .map(|option_amount| {
                if let Some(amount) = option_amount {
                    (!amount.is_zero()).then_some(amount)
                } else {
                    None
                }
            })
            .collect();
        Self {
            people,
            raw_amounts,
            total_amount: value.total_amount,
            paid_amounts,
        }
    }
}

impl GroupExpense {
    pub fn raw_total(&self) -> Decimal {
        self.raw_amounts.iter().sum()
    }

    pub fn total_amounts(&self) -> Vec<Decimal> {
        self.raw_amounts
            .iter()
            .map(|raw_amount| raw_amount * self.total_amount / self.raw_total())
            .collect()
    }
}

impl Expense for GroupExpense {
    fn record_template(records: &[Self], config: &KakeboConfig) -> String {
        let people: BTreeSet<&str> = records
            .iter()
            .flat_map(|record| record.people.iter())
            .map(|string_ref| string_ref.as_str())
            .collect();
        let other_people: Vec<&str> = people
            .into_iter()
            .filter(|name| *name != config.user_name.as_str())
            .collect();
        let other_people_string = other_people.join(" = 0.00\n");
        let other_people_string = if other_people_string.is_empty() {
            String::from("example_name")
        } else {
            other_people_string
        };
        format!(
            "total_amount = 0.00
{}

[raw_amounts]
# set values for existing people or add people manually
# deleting is not required, zero entries will be ignored
{} = 0.00
{existing_people} = 0.00

[paid_amounts]
# set values for existing people or add people manually
# deleting is not required, zero entries or people not in raw_amounts section will be ignored
{existing_people} = 0.00
",
            ExpenseKind::toml_template(),
            config.user_name,
            existing_people = other_people_string
        )
    }

    fn try_create(content: String) -> Result<Self, KakeboError> {
        let intermediate: GroupExpenseIntermediate = toml::from_str(&content)?;
        let record = GroupExpense::from(intermediate);
        Ok(record)
    }
}
