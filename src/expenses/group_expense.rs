use std::{collections::HashMap, fmt::Display};

use inquire::{Confirm, InquireError, Select};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{errors::KakeboError, Environment, KakeboConfig, ANSI_GREEN, ANSI_RED, ANSI_STOP};

use super::{money_amount, person, ExpenseInfo};

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

    pub fn new(environment: &Environment) -> Result<Self, KakeboError> {
        let info = ExpenseInfo::new()?;
        let raw_user_amount = money_amount(
            &environment.config,
            &format!("{} (raw)", &environment.config.user_name),
        )?;
        let mut people = Vec::new();
        let mut raw_amounts = Vec::new();

        loop {
            let person_name = person("Add person:", environment)?;
            if person_name.is_empty() {
                break;
            }
            let person_amount =
                money_amount(&environment.config, &format!("{} (raw)", person_name))?;
            people.push(person_name);
            raw_amounts.push(person_amount);
        }
        let total_amount = money_amount(&environment.config, "total")?;

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
            let paid_amount =
                money_amount(&environment.config, &format!("{} (paid)", person_that_paid))?;
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

    pub fn edit(&mut self, config: &KakeboConfig) -> Result<(), KakeboError> {
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
            .filter(|(_name, (_i, to_pay))| !to_pay.is_sign_negative())
            .collect();
        if need_to_pay.is_empty() {
            return Ok(());
        }
        loop {
            let options: Vec<String> = need_to_pay.keys().map(|&name| name.clone()).collect();
            let person_that_paid = Select::new("Who already payed?", options).prompt();
            if let Err(InquireError::OperationCanceled) = person_that_paid {
                return Ok(());
            }
            let person_that_paid = person_that_paid?;
            let paid_amount = money_amount(config, &format!("{} (paid)", person_that_paid))?;
            let index = need_to_pay
                .remove_entry(&person_that_paid)
                .ok_or_else(|| KakeboError::InvalidArgument(person_that_paid.clone()))?
                .1
                 .0;
            self.paid_amounts[index] = Some(paid_amount);
        }
    }
}
