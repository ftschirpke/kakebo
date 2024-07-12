use std::{
    cmp::Ordering,
    collections::{BTreeSet, HashMap},
    fmt::Display,
    fs::File,
    path::{Path, PathBuf},
    process::ExitCode,
};

use age::{secrecy::Secret, Decryptor, Encryptor};
use chrono::Local;
use chronoutil::RelativeDuration;
use clap::{Parser, Subcommand};
use expenses::{
    advancement::Advancement, debt::Debt, group_expense::GroupExpense,
    recurring_expense::RecurringExpense, single_expense::SingleExpense,
};
use inquire::{Confirm, Password, Select};
use lz4_flex::frame::{FrameDecoder, FrameEncoder};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use self::errors::KakeboError;

mod errors;
mod expenses;

const KAKEBO_DB_FILE: &str = "test.kakebo";

#[derive(Debug, Deserialize)]
pub struct KakeboConfig {
    pub currency: char,
    pub decimal_sep: char,
    pub user_name: String,
    pub database_dir: PathBuf,
}

fn parse_config() -> Result<KakeboConfig, KakeboError> {
    let cur_dir = std::env::current_dir()?;
    let config_path = cur_dir.join("kakebo.config");

    if !config_path.exists() {
        println!("No config file found at {}", config_path.display());
        println!(
            "Please create a config file. A minimal config would look like this:
user_name = \"Your name\"
currency = \"$\"
decimal_sep = \".\"
database_dir = \"/home/<username>/<path>/<to>/<dir>\""
        );
        return Err(KakeboError::InvalidArgument("No config file found".into()));
    }

    println!("Config file found at {}", config_path.display());
    let config = std::fs::read_to_string(config_path)?;
    let config: KakeboConfig = toml::from_str(&config)?;
    Ok(config)
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
    #[arg(short, long)]
    debug: bool,
}

#[derive(Subcommand, Debug)]
enum Command {
    Status {
        #[command(subcommand)]
        expense_type: Option<ExpenseType>,
    },
    Add {
        #[command(subcommand)]
        expense_type: ExpenseType,
    },
    Edit {
        #[command(subcommand)]
        expense_type: ExpenseType,
    },
    Delete {
        #[command(subcommand)]
        expense_type: ExpenseType,
    },
    Receive {
        value: Decimal,
        from: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum ExpenseType {
    Single,
    Group,
    Recurring,
    Todo,
    Advance,
}

#[derive(Subcommand, Debug)]
enum IncomeType {
    Single,
    Recurring,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Expenses {
    single_expenses: Vec<SingleExpense>,
    group_expenses: Vec<GroupExpense>,
    recurring_expenses: Vec<RecurringExpense>,
    debts_owed: Vec<Debt>,
    unpaid_advancements: Vec<Advancement>,
}

impl Expenses {
    pub fn new() -> Self {
        Self {
            single_expenses: Vec::new(),
            group_expenses: Vec::new(),
            recurring_expenses: Vec::new(),
            debts_owed: Vec::new(),
            unpaid_advancements: Vec::new(),
        }
    }

    pub fn print_status(&self) {
        println!("Expenses Overview:");
        let today = Local::now().date_naive();
        let month_ago = today - RelativeDuration::months(1);

        let single_expenses_last_month: Decimal = self
            .single_expenses
            .iter()
            .filter(|expense| expense.info.date > month_ago && expense.info.date <= today)
            .map(|expense| expense.amount)
            .sum();
        let group_expenses_last_month: Decimal = self
            .group_expenses
            .iter()
            .filter(|expense| expense.info.date > month_ago && expense.info.date <= today)
            .map(GroupExpense::true_user_amount)
            .sum::<Decimal>();
        let recurring_expenses_last_month: Decimal = self
            .recurring_expenses
            .iter()
            .map(|expense| expense.amount_in_interval(month_ago, today))
            .sum::<Decimal>();
        println!(
            "  Single Expenses last month:    {:8.2}",
            single_expenses_last_month
        );
        println!(
            "  Group Expenses last month:     {:8.2}",
            group_expenses_last_month
        );
        println!(
            "  Recurring Expenses last month: {:8.2}",
            recurring_expenses_last_month
        );
        println!(
            "  Total Expenses last month:     {:8.2}",
            single_expenses_last_month + group_expenses_last_month + recurring_expenses_last_month
        );

        println!("Balances:");
        let mut people_owe_user: HashMap<&str, Decimal> = HashMap::new();
        let mut user_owes_people: HashMap<&str, Decimal> = HashMap::new();

        let debts_from_group_expenses = self.group_expenses.iter().flat_map(|group_expense| {
            group_expense.people.iter().zip(
                group_expense
                    .true_amounts()
                    .into_iter()
                    .zip(group_expense.paid_amounts.iter()),
            )
        });
        for (person, (amount, already_paid)) in debts_from_group_expenses {
            let paid = already_paid.unwrap_or(Decimal::ZERO);
            match paid.cmp(&amount) {
                Ordering::Less => {
                    let owed: Decimal = amount - paid;
                    people_owe_user
                        .entry(person)
                        .and_modify(|val| *val += owed)
                        .or_insert(owed);
                }
                Ordering::Greater => {
                    let overpaid: Decimal = paid - amount;
                    user_owes_people
                        .entry(person)
                        .and_modify(|val| *val += overpaid)
                        .or_insert(overpaid);
                }
                Ordering::Equal => {}
            }
        }
        for debt in &self.debts_owed {
            user_owes_people
                .entry(&debt.person)
                .and_modify(|val| *val += debt.expense.amount)
                .or_insert(debt.expense.amount);
        }
        for advancement in &self.unpaid_advancements {
            people_owe_user
                .entry(&advancement.person)
                .and_modify(|val| *val += advancement.amount)
                .or_insert(advancement.amount);
        }

        let all_people: BTreeSet<_> = self
            .group_expenses
            .iter()
            .flat_map(|group| group.people.iter().map(String::as_str))
            .chain(self.debts_owed.iter().map(|debt| debt.person.as_str()))
            .chain(
                self.unpaid_advancements
                    .iter()
                    .map(|advc| advc.person.as_str()),
            )
            .collect();

        for person in all_people {
            let good = people_owe_user.get(person).map_or(Decimal::ZERO, |r| *r);
            let bad = user_owes_people.get(person).map_or(Decimal::ZERO, |r| *r);
            let balance = good - bad;
            println!(
                "  {:10} {ANSI_GREEN}{:8.2} {ANSI_RED}{:8.2}{ANSI_STOP}  TOTAL: {:+8.2}",
                person, good, bad, balance
            );
        }
    }
}

impl Default for Expenses {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct Environment {
    pub config: KakeboConfig,
    pub people: BTreeSet<String>,
}

fn parse_file<T>(path: &Path) -> Result<T, KakeboError>
where
    T: for<'de> Deserialize<'de> + Default,
{
    if !path.exists() {
        println!("File {} does not exist.", path.display());
        return Ok(T::default());
    }
    let passphrase = Password::new("Enter decryption password:")
        .with_display_mode(inquire::PasswordDisplayMode::Hidden)
        .without_confirmation()
        .prompt()?;
    let mut file = File::open(path)?;
    let decryptor = match Decryptor::new(&mut file)? {
        Decryptor::Passphrase(decr) => decr,
        _ => unreachable!(),
    };
    let mut decrypt_reader = decryptor.decrypt(&Secret::new(passphrase.to_owned()), None)?;
    let mut decode_reader = FrameDecoder::new(&mut decrypt_reader);
    let expenses = rmp_serde::decode::from_read(&mut decode_reader)?;
    println!("Expenses parsed from {}", path.display());
    Ok(expenses)
}

fn write_file<T>(path: &Path, expenses: &T) -> Result<(), KakeboError>
where
    T: Serialize,
{
    let passphrase = Password::new("Enter encryption password:")
        .with_display_mode(inquire::PasswordDisplayMode::Hidden)
        .without_confirmation()
        .prompt()?;
    let mut file = File::create(path)?;
    let encryptor = Encryptor::with_user_passphrase(Secret::new(passphrase.to_owned()));
    let mut encrypt_writer = encryptor.wrap_output(&mut file)?;
    let mut compress_writer = FrameEncoder::new(&mut encrypt_writer);
    rmp_serde::encode::write(&mut compress_writer, expenses)?;
    compress_writer.finish()?;
    encrypt_writer.finish()?;
    Ok(())
}

#[allow(dead_code)]
/// a function that simplifies tranforming older versions of the data structure into new ones
fn transform<Src, Dst>(path: &Path) -> Result<(), KakeboError>
where
    Src: for<'de> Deserialize<'de> + Default,
    Dst: Serialize + From<Src>,
{
    let from_content: Src = parse_file(path)?;
    let to_content: Dst = from_content.into();
    write_file(path, &to_content)
}

pub const ANSI_RED: &str = "\x1b[31m";
pub const ANSI_GREEN: &str = "\x1b[32m";
pub const ANSI_STOP: &str = "\x1b[0m";

trait DisplayableExpense: Display + Eq {
    fn name() -> &'static str;
    fn plural_name() -> &'static str;
    fn configured_display(&self, _config: &KakeboConfig) {
        println!("{}", self)
    }
}

fn status<T: DisplayableExpense>(expenses: &[T], config: &KakeboConfig) -> Result<(), KakeboError> {
    if expenses.is_empty() {
        println!("No {} to view.", T::plural_name());
        return Ok(());
    }
    let options: Vec<_> = expenses.iter().rev().collect();
    let to_view = Select::new(
        format!("Which {} do you want to view?", T::name()).as_str(),
        options,
    )
    .prompt()?;
    to_view.configured_display(config);
    Ok(())
}

fn delete<T: DisplayableExpense>(
    expenses: &mut Vec<T>,
    config: &KakeboConfig,
) -> Result<bool, KakeboError> {
    if expenses.is_empty() {
        println!("No {} to delete.", T::plural_name());
        return Ok(false);
    }
    let options: Vec<_> = expenses.iter().rev().collect();
    let to_delete = Select::new(
        format!("Which {} do you want to delete?", T::name()).as_str(),
        options,
    )
    .prompt()?;
    to_delete.configured_display(config);
    let idx = expenses
        .iter()
        .position(|adv| adv == to_delete)
        .unwrap_or_else(|| panic!("Selected {} must exist.", T::name()));
    let deletion_confirmed =
        Confirm::new(format!("Are you sure you want to delete this {}?", T::name()).as_str())
            .prompt()?;
    if deletion_confirmed {
        expenses.remove(idx);
    }
    Ok(deletion_confirmed)
}

fn run() -> Result<(), KakeboError> {
    let args = Args::parse();
    let config = parse_config()?;

    let path = config.database_dir.join(KAKEBO_DB_FILE);

    let mut expenses: Expenses = parse_file(&path)?;
    let mut environment = Environment {
        config,
        people: expenses
            .group_expenses
            .iter()
            .flat_map(|group| group.people.iter())
            .chain(expenses.debts_owed.iter().map(|debt| &debt.person))
            .chain(expenses.unpaid_advancements.iter().map(|advc| &advc.person))
            .map(String::clone)
            .collect(),
    };

    if args.debug {
        println!(
            "=== Expenses Before ===\n{:?}\n=======================",
            expenses
        );
    }

    let changes_made = match args.command {
        Command::Status { expense_type } => {
            match expense_type {
                None => {
                    println!("===== STATUS =====");
                    println!("User: {}", environment.config.user_name);
                    println!("Currency: {}", environment.config.currency);
                    expenses.print_status();
                    if args.debug {
                        println!("{:?}", expenses);
                    }
                }
                Some(ExpenseType::Single) => {
                    status(&expenses.single_expenses, &environment.config)?
                }
                Some(ExpenseType::Group) => status(&expenses.group_expenses, &environment.config)?,
                Some(ExpenseType::Recurring) => {
                    status(&expenses.recurring_expenses, &environment.config)?
                }
                Some(ExpenseType::Todo) => status(&expenses.debts_owed, &environment.config)?,
                Some(ExpenseType::Advance) => {
                    status(&expenses.unpaid_advancements, &environment.config)?
                }
            }
            false
        }
        Command::Delete { expense_type } => match expense_type {
            ExpenseType::Single => delete(&mut expenses.single_expenses, &environment.config)?,
            ExpenseType::Group => delete(&mut expenses.group_expenses, &environment.config)?,
            ExpenseType::Recurring => {
                delete(&mut expenses.recurring_expenses, &environment.config)?
            }
            ExpenseType::Todo => delete(&mut expenses.debts_owed, &environment.config)?,
            ExpenseType::Advance => delete(&mut expenses.unpaid_advancements, &environment.config)?,
        },
        Command::Add { expense_type } => {
            match expense_type {
                ExpenseType::Single => {
                    let single = SingleExpense::new(&environment.config)?;
                    if args.debug {
                        println!("{:?}", single);
                    }
                    expenses.single_expenses.push(single);
                }
                ExpenseType::Group => {
                    let group = GroupExpense::new(&environment)?;
                    if args.debug {
                        println!("{:?}", group);
                    }
                    environment
                        .people
                        .extend(group.people.iter().map(String::clone));
                    expenses.group_expenses.push(group);
                }
                ExpenseType::Recurring => {
                    let recurring = RecurringExpense::new(&environment.config)?;
                    if args.debug {
                        println!("{:?}", recurring);
                    }
                    expenses.recurring_expenses.push(recurring);
                }
                ExpenseType::Todo => {
                    let debt = Debt::new(&environment)?;
                    if args.debug {
                        println!("{:?}", debt);
                    }
                    environment.people.insert(debt.person.clone());
                    expenses.debts_owed.push(debt);
                }
                ExpenseType::Advance => {
                    let advancement = Advancement::new(&environment)?;
                    if args.debug {
                        println!("{:?}", advancement);
                    }
                    environment.people.insert(advancement.person.clone());
                    expenses.unpaid_advancements.push(advancement);
                }
            }
            true
        }
        Command::Edit { expense_type } => {
            println!("Editing...");
            match expense_type {
                ExpenseType::Single => todo!("Edit single expenses"),
                ExpenseType::Group => {
                    if expenses.group_expenses.is_empty() {
                        println!("No {} to edit.", GroupExpense::plural_name());
                        false
                    } else {
                        let options: Vec<_> = expenses.group_expenses.iter_mut().rev().collect();
                        let to_edit =
                            Select::new("Which group expense do you want to edit?", options)
                                .prompt()?;
                        to_edit.edit(&environment.config)?
                    }
                }
                ExpenseType::Recurring => todo!("Edit recurring expenses"),
                ExpenseType::Todo => {
                    if expenses.debts_owed.is_empty() {
                        println!("No {} to edit.", Debt::plural_name());
                        false
                    } else {
                        let options: Vec<_> = expenses.debts_owed.iter().rev().collect();
                        let to_edit =
                            Select::new("Which debt do you want to edit?", options).prompt()?;
                        println!("{}", to_edit);
                        let payed_up = Confirm::new(&format!(
                            "Have you paid {} back the {}{}?",
                            to_edit.person, to_edit.expense.amount, environment.config.currency
                        ))
                        .prompt()?;
                        if payed_up {
                            let index = expenses
                                .debts_owed
                                .iter()
                                .position(|debt| debt == to_edit)
                                .expect("The debt we edit must exist");
                            let debt_paid = expenses.debts_owed.remove(index);
                            expenses.single_expenses.push(debt_paid.expense);
                        }
                        payed_up
                    }
                }
                ExpenseType::Advance => {
                    if expenses.unpaid_advancements.is_empty() {
                        println!("No {} to edit.", Advancement::plural_name());
                        false
                    } else {
                        let options: Vec<_> = expenses.unpaid_advancements.iter().rev().collect();
                        let to_edit =
                            Select::new("Which debt do you want to edit?", options).prompt()?;
                        println!("{}", to_edit);
                        let payed_up = Confirm::new(&format!(
                            "Has {} paid you back the {}{}?",
                            to_edit.person, to_edit.amount, environment.config.currency
                        ))
                        .prompt()?;
                        if payed_up {
                            let index = expenses
                                .unpaid_advancements
                                .iter()
                                .position(|advancement| advancement == to_edit)
                                .expect("The advancement we edit must exist");
                            expenses.unpaid_advancements.remove(index);
                        }
                        payed_up
                    }
                }
            }
        }
        Command::Receive { value, from } => {
            if let Some(src) = from {
                todo!("Receive {} from {}", value, src)
            } else {
                todo!("Receive {}", value);
            }
        }
    };

    if !changes_made {
        return Ok(());
    }

    if args.debug {
        println!(
            "=== Expenses After ===\n{:?}\n======================",
            expenses
        );
    }

    write_file(&path, &expenses)
}

fn main() -> ExitCode {
    let result = run();
    if let Err(error) = result {
        println!("{}", error);
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
