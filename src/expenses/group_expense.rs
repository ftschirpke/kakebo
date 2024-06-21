use std::collections::{BTreeSet, HashMap};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{errors::KakeboError, KakeboConfig};

use super::{confirm, money_amount, Expense, ExpenseKind};

#[derive(Debug)]
/// a group expense that distributes extra costs such as tips or delivery fees fairly
pub struct GroupExpense {
    people: Vec<String>,
    raw_amounts: Vec<Decimal>,
    total_amount: Decimal,
    paid_amounts: Vec<Option<Decimal>>,
    kind: ExpenseKind,
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

    pub fn new(config: &KakeboConfig) -> Result<Self, KakeboError> {
        let kind = ExpenseKind::new()?;
        let amount = money_amount(config)?;
        let confimation = confirm()?;
    }
}
