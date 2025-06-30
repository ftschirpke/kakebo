use std::{collections::HashMap, fmt::Display};

use inquire::{Confirm, InquireError, Select};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{
    errors::KakeboError, DisplayableExpense, Environment, KakeboConfig, ANSI_GREEN, ANSI_RED,
    ANSI_STOP,
};

use super::{money_amount, person, ExpenseInfo, NEW_PERSON};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupExpensePart {
    pub index: usize,
    pub info: ExpenseInfo,
    pub person: String,
    pub paid: Option<Decimal>,
    pub to_pay: Decimal,
}

impl Display for GroupExpensePart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} - {} {:8.2} (paid: {:8.2})",
            self.info,
            self.person,
            self.to_pay,
            self.paid.unwrap_or(Decimal::ZERO)
        )
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
/// a group expense that distributes extra costs such as tips or delivery fees fairly
pub struct GroupExpense {
    pub info: ExpenseInfo,
    raw_user_amount: Decimal,
    pub people: Vec<String>,
    raw_amounts: Vec<Decimal>,
    total_amount: Decimal,
    pub paid_amounts: Vec<Option<Decimal>>,
}

impl Display for GroupExpense {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (Total: {:8.2})", self.info, self.total_amount)
    }
}

impl DisplayableExpense for GroupExpense {
    fn name() -> &'static str {
        "group expense"
    }

    fn plural_name() -> &'static str {
        "group expenses"
    }

    fn configured_display(&self, config: &KakeboConfig) {
        self.print(config)
    }
}

impl GroupExpense {
    pub fn raw_total(&self) -> Decimal {
        self.raw_amounts.iter().sum::<Decimal>() + self.raw_user_amount
    }

    fn true_amount(&self, raw_amount: Decimal) -> Decimal {
        let true_amount_unscaled = raw_amount * self.total_amount / self.raw_total();
        true_amount_unscaled.round_dp(2)
    }

    pub fn true_user_amount(&self) -> Decimal {
        self.true_amount(self.raw_user_amount)
    }

    pub fn parts(&self) -> impl Iterator<Item = GroupExpensePart> + use<'_> {
        self.people
            .iter()
            .zip(self.true_amounts())
            .zip(&self.paid_amounts)
            .enumerate()
            .map(|(i, ((person, to_pay), paid))| GroupExpensePart {
                index: i,
                info: self.info.clone(),
                person: person.to_string(),
                paid: *paid,
                to_pay,
            })
    }

    pub fn true_amounts(&self) -> Vec<Decimal> {
        self.raw_amounts
            .iter()
            .map(|&raw_amount| self.true_amount(raw_amount))
            .collect()
    }

    pub fn print(&self, config: &KakeboConfig) {
        println!("{}", self);
        for (person, (needed, paid)) in self.people.iter().zip(
            self.true_amounts()
                .into_iter()
                .zip(self.paid_amounts.iter()),
        ) {
            let paid = paid.unwrap_or(Decimal::ZERO);
            let to_pay = needed - paid;
            let to_pay_colour = if to_pay.is_sign_negative() {
                ANSI_GREEN
            } else {
                ANSI_RED
            };
            println!(
                "  {:10} {to_pay_colour}{:8.2}{currency}{ANSI_STOP} (to pay: {:8.2}{currency}, paid: {:8.2}{currency})",
                person, to_pay, needed, paid, currency=config.currency
            );
        }
    }

    pub fn new(environment: &Environment, config: &KakeboConfig) -> Result<Self, KakeboError> {
        let info = ExpenseInfo::new()?;
        let raw_user_amount = money_amount(config, &format!("{} (raw)", &config.user_name))?;
        let mut people = Vec::new();
        let mut raw_amounts = Vec::new();

        let mut people_still_possible = environment.people.clone();

        loop {
            let person_result = person("Add person:", &people_still_possible);
            if let Err(InquireError::OperationCanceled) = person_result {
                break;
            }
            let person_name = person_result?;
            if person_name == NEW_PERSON {
                continue;
            }
            let person_amount = money_amount(config, &format!("{} (raw)", person_name))?;
            people_still_possible.remove(&person_name);
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

        let new_instance = Self {
            info,
            raw_user_amount,
            people,
            raw_amounts,
            total_amount,
            paid_amounts,
        };
        new_instance.configured_display(config);

        if Confirm::new("Save this expense?").prompt()? {
            Ok(new_instance)
        } else {
            Err(KakeboError::ExpenseCreationAborted)
        }
    }

    pub fn edit(&mut self, config: &KakeboConfig) -> Result<bool, KakeboError> {
        self.print(config);

        let to_pay = self
            .true_amounts()
            .into_iter()
            .zip(self.paid_amounts.iter())
            .map(|(to_pay, paid)| {
                if let Some(paid) = paid {
                    to_pay - paid
                } else {
                    to_pay
                }
            });
        let mut need_to_pay: HashMap<&String, (usize, Decimal)> = self
            .people
            .iter()
            .zip(to_pay.enumerate())
            .filter(|(_name, (_i, to_pay))| to_pay.is_sign_positive() && !to_pay.is_zero())
            .collect();
        if need_to_pay.is_empty() {
            return Ok(false);
        }
        let mut changes_made = false;
        while !need_to_pay.is_empty() {
            let options: Vec<String> = need_to_pay.keys().map(|&name| name.clone()).collect();
            let person_that_paid = Select::new("Who already payed?", options).prompt();
            if let Err(InquireError::OperationCanceled) = person_that_paid {
                return Ok(changes_made);
            }
            let person_that_paid = person_that_paid?;
            let paid_amount = money_amount(config, &format!("{} (paid)", person_that_paid))?;
            let index = need_to_pay
                .remove_entry(&person_that_paid)
                .ok_or_else(|| KakeboError::InvalidArgument(person_that_paid.clone()))?
                .1
                 .0;
            self.paid_amounts[index] = Some(paid_amount);
            changes_made = true;
        }
        Ok(changes_made)
    }
}
