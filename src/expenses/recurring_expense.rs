use std::fmt::Display;

use chrono::NaiveDate;
use chrono::Weekday;
use chronoutil::RelativeDuration;
use inquire::validator::Validation;
use inquire::Confirm;
use inquire::CustomType;
use inquire::DateSelect;
use inquire::Select;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde::Serialize;

use crate::errors::KakeboError;
use crate::DisplayableExpense;
use crate::KakeboConfig;

use super::money_amount;
use super::ExpenseInfo;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecurringExpense {
    pub amount: Decimal,
    pub info: ExpenseInfo,
    every: DateDelta,
    end_date: Option<NaiveDate>,
}

impl Display for RecurringExpense {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let end = if let Some(e) = self.end_date {
            format!("until {}", e.clone())
        } else {
            "without end".to_string()
        };
        write!(
            f,
            "{} ({:8.2} every {} {})",
            self.info, self.amount, self.every, end
        )
    }
}

impl DisplayableExpense for RecurringExpense {
    fn name() -> &'static str {
        "recurring expense"
    }

    fn plural_name() -> &'static str {
        "recurring expenses"
    }
}

impl RecurringExpense {
    pub fn amount_in_interval(&self, start: NaiveDate, end: NaiveDate) -> Decimal {
        if self.info.date > end {
            return Decimal::ZERO;
        }
        if let Some(end_date) = self.end_date {
            if end_date < start {
                return Decimal::ZERO;
            }
        }

        let cycle = RelativeDuration::from(&self.every);

        let mut date = self.info.date;
        while date < start {
            date = date + cycle;
        }

        if let Some(end_date) = self.end_date {
            if end_date < date {
                return Decimal::ZERO;
            }
        }

        let mut total_amount = Decimal::ZERO;
        while date <= end {
            if let Some(end_date) = self.end_date {
                if end_date < date {
                    return total_amount;
                }
            }
            total_amount += self.amount;
            date = date + cycle;
        }
        total_amount
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
enum DateDelta {
    Days(u8),
    Weeks(u8),
    Months(u8),
    Years(u8),
}

impl Display for DateDelta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            DateDelta::Days(1) => write!(f, "every day"),
            DateDelta::Days(n) => write!(f, "every {} days", n),
            DateDelta::Weeks(1) => write!(f, "every week"),
            DateDelta::Weeks(n) => write!(f, "every {} weeks", n),
            DateDelta::Months(1) => write!(f, "every month"),
            DateDelta::Months(n) => write!(f, "every {} months", n),
            DateDelta::Years(1) => write!(f, "every year"),
            DateDelta::Years(n) => write!(f, "every {} years", n),
        }
    }
}

impl From<&DateDelta> for RelativeDuration {
    fn from(value: &DateDelta) -> Self {
        match *value {
            DateDelta::Days(days) => RelativeDuration::days(days as i64),
            DateDelta::Weeks(weeks) => RelativeDuration::weeks(weeks as i64),
            DateDelta::Months(months) => RelativeDuration::months(months as i32),
            DateDelta::Years(years) => RelativeDuration::years(years as i32),
        }
    }
}

impl RecurringExpense {
    pub fn new(config: &KakeboConfig) -> Result<Self, KakeboError> {
        let cycle_units: Vec<&str> = vec!["Day(s)", "Week(s)", "Month(s)", "Year(s)"];

        let info = ExpenseInfo::new()?;
        let amount = money_amount(config, &config.user_name)?;

        let cycle_unit = Select::new("How often does this repeat? (unit)", cycle_units).prompt()?;
        let cycle_amount = CustomType::<u8>::new("How often does this repeat? (amount)")
            .with_validator(|&input: &u8| {
                if input == 0 {
                    Ok(Validation::Invalid(
                        "Repeating interval must be positive (non-zero).".into(),
                    ))
                } else {
                    Ok(Validation::Valid)
                }
            })
            .with_error_message("Please type a valid positive number")
            .with_formatter(&|amount| format!("Every {} {}", amount, cycle_unit))
            .prompt()?;

        let every = match cycle_unit {
            "Day(s)" => DateDelta::Days(cycle_amount),
            "Week(s)" => DateDelta::Weeks(cycle_amount),
            "Month(s)" => DateDelta::Months(cycle_amount),
            "Year(s)" => DateDelta::Years(cycle_amount),
            _ => unreachable!(),
        };

        let has_end = Confirm::new("Does this recurring expense have an end date?").prompt()?;
        let end_date = if has_end {
            Some(
                DateSelect::new("Date:")
                    .with_week_start(Weekday::Mon)
                    .prompt()?,
            )
        } else {
            None
        };

        let new_instance = Self {
            info,
            amount,
            every,
            end_date,
        };
        new_instance.configured_display(config);

        if Confirm::new("Save this expense?").prompt()? {
            Ok(new_instance)
        } else {
            Err(KakeboError::ExpenseCreationAborted)
        }
    }
}
