use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs::File,
    path::Path,
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
use inquire::Password;
use lz4_flex::frame::{FrameDecoder, FrameEncoder};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use self::errors::KakeboError;

mod errors;
mod expenses;

#[derive(Debug, Deserialize)]
pub struct KakeboConfig {
    pub currency: char,
    pub decimal_sep: char,
    pub user_name: String,
}

fn parse_config() -> Result<KakeboConfig, KakeboError> {
    let cur_dir = std::env::current_dir()?;
    let config_path = cur_dir.join("kakebo.config");

    if !config_path.exists() {
        println!("No config file found at {}", config_path.display());
        println!(
            "Please create a config file. A minimal config would look like this:
\"user_name\" = \"Your name\"
\"currency\" = \"$\"
\"decimal_sep\" = \".\""
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
    Status,
    Edit,
    Add {
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
}

impl Default for Expenses {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_file<T>(path: &Path) -> Result<T, KakeboError>
where
    T: for<'de> Deserialize<'de> + Default,
{
    if !path.exists() {
        return Ok(T::default());
    }
    let mut file = File::open(path)?;
    let passphrase = Password::new("Enter decryption password:")
        .with_display_mode(inquire::PasswordDisplayMode::Hidden)
        .without_confirmation()
        .prompt()?;
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
    let mut file = File::create(path)?;
    let passphrase = Password::new("Enter encryption password:")
        .with_display_mode(inquire::PasswordDisplayMode::Hidden)
        .without_confirmation()
        .prompt()?;
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

const KAKEBO_DB_FILE: &str = "test.kakebo";
const ANSI_RED: &str = "\x1b[31m";
const ANSI_GREEN: &str = "\x1b[32m";
const ANSI_STOP: &str = "\x1b[0m";

fn run() -> Result<(), KakeboError> {
    let args = Args::parse();
    let config = parse_config()?;

    let cur_dir = std::env::current_dir()?;
    // let path = cur_dir.join(DEBUG_DB_FILE);
    let path = cur_dir.join(KAKEBO_DB_FILE);

    let mut expenses: Expenses = parse_file(&path)?;

    if args.debug {
        println!(
            "=== Expenses Before ===\n{:?}\n=======================",
            expenses
        );
    }

    let changes_made = match args.command {
        Command::Status => {
            println!("===== STATUS =====");
            println!("User: {}", config.user_name);
            println!("Currency: {}", config.currency);

            println!("Expenses Overview:");
            let today = Local::now().date_naive();
            let month_ago = today - RelativeDuration::months(1);

            let single_expenses_last_month: Decimal = expenses
                .single_expenses
                .iter()
                .filter(|expense| expense.info.date > month_ago && expense.info.date <= today)
                .map(|expense| expense.amount)
                .sum();
            let group_expenses_last_month: Decimal = expenses
                .group_expenses
                .iter()
                .filter(|expense| expense.info.date > month_ago && expense.info.date <= today)
                .map(GroupExpense::true_user_amount)
                .sum::<Decimal>();
            let recurring_expenses_last_month: Decimal = expenses
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

            println!("Balances:");
            let mut people_owe_user: HashMap<&str, Decimal> = HashMap::new();
            let mut user_owes_people: HashMap<&str, Decimal> = HashMap::new();

            let debts_from_group_expenses =
                expenses.group_expenses.iter().flat_map(|group_expense| {
                    group_expense.people.iter().zip(
                        group_expense
                            .true_amounts()
                            .into_iter()
                            .zip(group_expense.paid_amounts.iter()),
                    )
                });
            for (person, (amount, already_paid)) in debts_from_group_expenses {
                if let Some(paid) = already_paid {
                    if *paid < amount {
                        let owed: Decimal = amount - paid;
                        people_owe_user
                            .entry(person)
                            .and_modify(|val| *val += owed)
                            .or_insert(owed);
                    } else if *paid > amount {
                        let overpaid: Decimal = paid - amount;
                        user_owes_people
                            .entry(person)
                            .and_modify(|val| *val += overpaid)
                            .or_insert(overpaid);
                    }
                } else {
                    people_owe_user
                        .entry(person)
                        .and_modify(|val| *val += amount)
                        .or_insert(amount);
                }
            }
            for debt in &expenses.debts_owed {
                user_owes_people
                    .entry(&debt.person)
                    .and_modify(|val| *val += debt.expense.amount)
                    .or_insert(debt.expense.amount);
            }
            for advancement in &expenses.unpaid_advancements {
                people_owe_user
                    .entry(&advancement.person)
                    .and_modify(|val| *val += advancement.amount)
                    .or_insert(advancement.amount);
            }

            let all_people: BTreeSet<_> = expenses
                .group_expenses
                .iter()
                .flat_map(|group| group.people.iter())
                .chain(expenses.debts_owed.iter().map(|debt| &debt.person))
                .chain(expenses.unpaid_advancements.iter().map(|advc| &advc.person))
                .collect();

            for person in all_people {
                let good = people_owe_user
                    .get(person.as_str())
                    .map_or(Decimal::ZERO, |r| *r);
                let bad = user_owes_people
                    .get(person.as_str())
                    .map_or(Decimal::ZERO, |r| *r);
                let balance = good - bad;
                println!(
                    "  {:10} {ANSI_GREEN}{:8.2} {ANSI_RED}{:8.2}{ANSI_STOP}  TOTAL: {:+8.2}",
                    person, good, bad, balance
                );
            }

            if args.debug {
                println!("{:?}", expenses)
            }
            false
        }
        Command::Edit => {
            println!("Edit...");
            false
        }
        Command::Add { expense_type } => {
            match expense_type {
                ExpenseType::Single => {
                    let single = SingleExpense::new(&config)?;
                    if args.debug {
                        println!("{:?}", single);
                    }
                    expenses.single_expenses.push(single);
                }
                ExpenseType::Group => {
                    let group = GroupExpense::new(&config)?;
                    if args.debug {
                        println!("{:?}", group);
                    }
                    expenses.group_expenses.push(group);
                }
                ExpenseType::Recurring => {
                    let recurring = RecurringExpense::new(&config)?;
                    if args.debug {
                        println!("{:?}", recurring);
                    }
                    expenses.recurring_expenses.push(recurring);
                }
                ExpenseType::Todo => {
                    let debt = Debt::new(&config)?;
                    if args.debug {
                        println!("{:?}", debt);
                    }
                    expenses.debts_owed.push(debt);
                }
                ExpenseType::Advance => {
                    let advancement = Advancement::new(&config)?;
                    if args.debug {
                        println!("{:?}", advancement);
                    }
                    expenses.unpaid_advancements.push(advancement);
                }
            }
            true
        }
        Command::Receive { value, from } => {
            if let Some(src) = from {
                println!("Receive {} from {}", value, src);
            } else {
                println!("Receive {}", value);
            }
            false
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

    let path = Path::new(KAKEBO_DB_FILE);
    write_file(path, &expenses)
}

fn main() -> ExitCode {
    let result = run();
    if let Err(error) = result {
        println!("{}", error);
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
