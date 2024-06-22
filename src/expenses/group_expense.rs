use std::collections::HashMap;

use inquire::{Confirm, InquireError, Select, Text};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{errors::KakeboError, KakeboConfig};

use super::{money_amount, ExpenseInfo};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
/// a group expense that distributes extra costs such as tips or delivery fees fairly
pub struct GroupExpense {
    info: ExpenseInfo,
    raw_user_amount: Decimal,
    people: Vec<String>,
    raw_amounts: Vec<Decimal>,
    total_amount: Decimal,
    paid_amounts: Vec<Option<Decimal>>,
}

impl GroupExpense {
    pub fn raw_total(&self) -> Decimal {
        self.raw_amounts.iter().sum::<Decimal>() + self.raw_user_amount
    }

    fn true_amount(&self, raw_amount: Decimal) -> Decimal {
        raw_amount * self.total_amount / self.raw_total()
    }

    pub fn true_user_amount(&self) -> Decimal {
        self.true_amount(self.raw_user_amount)
    }

    pub fn true_amounts(&self) -> Vec<Decimal> {
        self.raw_amounts
            .iter()
            .map(|&raw_amount| self.true_amount(raw_amount))
            .collect()
    }

    pub fn new(config: &KakeboConfig) -> Result<Self, KakeboError> {
        let info = ExpenseInfo::new()?;
        let raw_user_amount = money_amount(config, &format!("{} (raw)", config.user_name))?;
        let mut people = Vec::new();
        let mut raw_amounts = Vec::new();

        loop {
            let person_name = Text::new("Add person:") // TODO: suggest most commonly used names
                .prompt()?;
            if person_name.is_empty() {
                break;
            }
            let person_amount = money_amount(config, &format!("{} (raw)", person_name))?;
            people.push(person_name);
            raw_amounts.push(person_amount);
        }
        let total_amount = money_amount(config, "total")?;

        let mut paid_amounts = vec![None; people.len()];
        let mut need_to_pay: HashMap<String, usize> = people
            .iter()
            .enumerate()
            .map(|(i, person)| (person.clone(), i))
            .collect();

        while !need_to_pay.is_empty() {
            let options: Vec<String> = need_to_pay.keys().map(String::clone).collect();
            let person_that_paid = Select::new("Who already payed?", options).prompt();
            if let Err(InquireError::OperationCanceled) = person_that_paid {
                break;
            }
            let person_that_paid = person_that_paid?;
            let paid_amount = money_amount(config, &format!("{} (paid)", person_that_paid))?;
            let index = need_to_pay
                .remove_entry(&person_that_paid)
                .ok_or_else(|| KakeboError::InvalidArgument(person_that_paid.clone()))?
                .1;
            paid_amounts[index] = Some(paid_amount);
        }

        if Confirm::new("Save this expense?").prompt()? {
            Ok(Self {
                info,
                raw_user_amount,
                people,
                raw_amounts,
                total_amount,
                paid_amounts,
            })
        } else {
            Err(KakeboError::ExpenseCreationAborted)
        }
    }
}
