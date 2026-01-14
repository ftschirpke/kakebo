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
use inquire::{Confirm, Password, Select};
use lz4_flex::frame::{FrameDecoder, FrameEncoder};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use self::errors::KakeboError;
use expenses::{
    advancement::Advancement, debt::Debt, group_expense::GroupExpense,
    recurring_expense::RecurringExpense, single_expense::SingleExpense,
};

mod errors;
mod expenses;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct KakeboConfig {
    pub currency: char,
    pub decimal_sep: char,
    pub user_name: String,
}

impl Default for KakeboConfig {
    fn default() -> Self {
        Self {
            currency: '€',
            decimal_sep: '.',
            user_name: "Friedrich".to_string(),
        }
    }
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
    Pstatus {
        person: String,
    },
    List {
        person: String,
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
    Sanitize,
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

#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Expenses {
    config: KakeboConfig,
    single_expenses: Vec<SingleExpense>,
    group_expenses: Vec<GroupExpense>,
    recurring_expenses: Vec<RecurringExpense>,
    debts_owed: Vec<Debt>,
    unpaid_advancements: Vec<Advancement>,
    overflows: HashMap<String, Decimal>,
}

impl Expenses {
    pub fn all_people(&self) -> impl Iterator<Item = String> + use<'_> {
        self.group_expenses
            .iter()
            .flat_map(|group| group.people.iter())
            .chain(self.debts_owed.iter().map(|debt| &debt.person))
            .chain(self.unpaid_advancements.iter().map(|advc| &advc.person))
            .chain(self.overflows.keys())
            .map(String::clone)
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
        for (person, overflow) in &self.overflows {
            user_owes_people
                .entry(person)
                .and_modify(|val| *val += overflow)
                .or_insert(*overflow);
        }

        let all_people: BTreeSet<_> = self.all_people().collect();
        println!("             they owe   you owe          balance");
        for person in all_people {
            let good = people_owe_user
                .get(person.as_str())
                .map_or(Decimal::ZERO, |r| *r);
            let bad = user_owes_people
                .get(person.as_str())
                .map_or(Decimal::ZERO, |r| *r);
            let balance = good - bad;
            let balance_color = if Decimal::abs(&balance) >= Decimal::from(5) {
                ANSI_RED
            } else {
                ANSI_GREEN
            };
            println!(
                "  {:10} {:8.2}, {:8.2}, TOTAL: {balance_color}{:+8.2}{ANSI_STOP}",
                person, good, bad, balance
            );
        }
    }
}

#[derive(Debug)]
pub struct Environment {
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

struct DisplayPath {
    inner: PathBuf,
}

impl Display for DisplayPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(inner_str) = self.inner.to_str() {
            write!(f, "{}", inner_str)
        } else {
            write!(f, "Could not resolve path")
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PathCache {
    data: Vec<String>,
}

fn run() -> Result<(), KakeboError> {
    let args = Args::parse();

    let search_dir = dirs::home_dir().expect("Resolve home directory");

    let cache_path = search_dir.join(".kakebo-cache");
    let cached_possible_paths: Option<Vec<_>> = if cache_path.exists() {
        let cache = std::fs::read_to_string(&cache_path)?;
        let possible_paths: PathCache = toml::from_str(&cache)?;
        Some(
            possible_paths
                .data
                .into_iter()
                .map(|s| DisplayPath {
                    inner: Path::new(&s).to_owned(),
                })
                .collect(),
        )
    } else {
        None
    };

    let cache_is_valid = if let Some(cached_paths) = &cached_possible_paths {
        cached_paths
            .iter()
            .all(|display_path| display_path.inner.exists())
    } else {
        false
    };

    let mut possible_paths = if cache_is_valid {
        cached_possible_paths.unwrap()
    } else {
        let mut possible_paths = Vec::new();
        for entry in WalkDir::new(search_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let filename = entry.file_name().to_string_lossy();

            if filename.ends_with(".kakebo") {
                possible_paths.push(DisplayPath {
                    inner: entry.path().to_owned(),
                });
            }
        }
        let cache: Vec<String> = possible_paths
            .iter()
            .map(|display_path| display_path.inner.to_string_lossy().into_owned())
            .collect();
        let cache = PathCache { data: cache };
        let cache_str = toml::to_string_pretty(&cache)?;
        std::fs::write(&cache_path, cache_str)?;
        possible_paths
    };

    let path = match possible_paths.len() {
        0 => {
            return Err(KakeboError::InvalidArgument(
                "Could not find any database".to_string(),
            ))
        }
        1 => possible_paths.pop().unwrap(),
        _ => Select::new(
            "Which Kakebo database do you want to access?",
            possible_paths,
        )
        .prompt()?,
    };
    let path = Path::new(&path.inner);

    // NOTE: this is a way to update the file format, I leave this as reference
    // transform::<OldExpenses, Expenses>(path)?;
    // return Ok(());

    let mut expenses: Expenses = parse_file(path)?;
    let mut environment = Environment {
        people: expenses.all_people().collect(),
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
                    println!("User: {}", expenses.config.user_name);
                    println!("Currency: {}", expenses.config.currency);
                    expenses.print_status();
                    if args.debug {
                        println!("{:?}", expenses);
                    }
                }
                Some(ExpenseType::Single) => status(&expenses.single_expenses, &expenses.config)?,
                Some(ExpenseType::Group) => status(&expenses.group_expenses, &expenses.config)?,
                Some(ExpenseType::Recurring) => {
                    status(&expenses.recurring_expenses, &expenses.config)?
                }
                Some(ExpenseType::Todo) => status(&expenses.debts_owed, &expenses.config)?,
                Some(ExpenseType::Advance) => {
                    status(&expenses.unpaid_advancements, &expenses.config)?
                }
            }
            false
        }
        Command::Pstatus { person } => {
            if expenses.all_people().all(|p| p != person) {
                return Err(KakeboError::InvalidArgument(format!(
                    "{} is not a known person",
                    person
                )));
            }
            let debts_from_group_expenses =
                expenses.group_expenses.iter().flat_map(|group_expense| {
                    group_expense.parts().filter(|part| {
                        part.person == person && part.to_pay != part.paid.unwrap_or(Decimal::ZERO)
                    })
                });

            let mut total_owed = Decimal::ZERO;
            let mut total_overflow = Decimal::ZERO;

            for part in debts_from_group_expenses {
                println!("  {}", part);
                let paid = part.paid.unwrap_or(Decimal::ZERO);
                match paid.cmp(&part.to_pay) {
                    Ordering::Less => {
                        let owed = part.to_pay - paid;
                        total_owed += owed;
                    }
                    Ordering::Greater => {
                        let overpaid = paid - part.to_pay;
                        total_overflow += overpaid
                    }
                    Ordering::Equal => {}
                }
            }
            // TODO: debts owed and advancements
            let overflow = expenses.overflows.get(&person).unwrap_or(&Decimal::ZERO);
            total_overflow += overflow;
            println!("  {} {:8.2} overflow", person, overflow);
            println!("             they owe   you owe          balance");
            let balance = total_owed - total_overflow;
            let balance_color = if Decimal::abs(&balance) >= Decimal::from(5) {
                ANSI_RED
            } else {
                ANSI_GREEN
            };
            println!(
                "  {:10} {:8.2}, {:8.2}, TOTAL: {balance_color}{:+8.2}{ANSI_STOP}",
                person, total_owed, total_overflow, balance
            );

            false
        }
        Command::List { person } => {
            if expenses.all_people().all(|p| p != person) {
                return Err(KakeboError::InvalidArgument(format!(
                    "{} is not a known person",
                    person
                )));
            }

            let all_expenses = expenses.group_expenses.iter().flat_map(|group_expense| {
                group_expense.parts().filter(|part| part.person == person)
            });

            let mut total_owed = Decimal::ZERO;
            let mut total_overflow = Decimal::ZERO;

            for part in all_expenses {
                println!("  {}", part);
                let paid = part.paid.unwrap_or(Decimal::ZERO);
                match paid.cmp(&part.to_pay) {
                    Ordering::Less => {
                        let owed = part.to_pay - paid;
                        total_owed += owed;
                    }
                    Ordering::Greater => {
                        let overpaid = paid - part.to_pay;
                        total_overflow += overpaid
                    }
                    Ordering::Equal => {}
                }
            }
            // TODO: debts owed and advancements
            let overflow = expenses.overflows.get(&person).unwrap_or(&Decimal::ZERO);
            total_overflow += overflow;
            println!("  {} {:8.2} overflow", person, overflow);
            println!("             they owe   you owe          balance");
            let balance = total_owed - total_overflow;
            let balance_color = if Decimal::abs(&balance) >= Decimal::from(5) {
                ANSI_RED
            } else {
                ANSI_GREEN
            };
            println!(
                "  {:10} {:8.2}, {:8.2}, TOTAL: {balance_color}{:+8.2}{ANSI_STOP}",
                person, total_owed, total_overflow, balance
            );

            false
        }
        Command::Delete { expense_type } => match expense_type {
            ExpenseType::Single => delete(&mut expenses.single_expenses, &expenses.config)?,
            ExpenseType::Group => delete(&mut expenses.group_expenses, &expenses.config)?,
            ExpenseType::Recurring => delete(&mut expenses.recurring_expenses, &expenses.config)?,
            ExpenseType::Todo => delete(&mut expenses.debts_owed, &expenses.config)?,
            ExpenseType::Advance => delete(&mut expenses.unpaid_advancements, &expenses.config)?,
        },
        Command::Add { expense_type } => {
            match expense_type {
                ExpenseType::Single => {
                    let single = SingleExpense::new(&expenses.config)?;
                    if args.debug {
                        println!("{:?}", single);
                    }
                    expenses.single_expenses.push(single);
                }
                ExpenseType::Group => {
                    let group = GroupExpense::new(&environment, &expenses.config)?;
                    if args.debug {
                        println!("{:?}", group);
                    }
                    environment
                        .people
                        .extend(group.people.iter().map(String::clone));
                    expenses.group_expenses.push(group);
                }
                ExpenseType::Recurring => {
                    let recurring = RecurringExpense::new(&expenses.config)?;
                    if args.debug {
                        println!("{:?}", recurring);
                    }
                    expenses.recurring_expenses.push(recurring);
                }
                ExpenseType::Todo => {
                    let debt = Debt::new(&environment, &expenses.config)?;
                    if args.debug {
                        println!("{:?}", debt);
                    }
                    environment.people.insert(debt.person.clone());
                    expenses.debts_owed.push(debt);
                }
                ExpenseType::Advance => {
                    let advancement = Advancement::new(&environment, &expenses.config)?;
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
                        to_edit.edit(&expenses.config)?
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
                            to_edit.person, to_edit.expense.amount, expenses.config.currency
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
                            to_edit.person, to_edit.amount, expenses.config.currency
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
            let source_person = if let Some(src) = from {
                if environment.people.contains(&src) {
                    src
                } else {
                    return Err(KakeboError::InvalidArgument(format!(
                        "Cannot receive {} from {}, {} does not exist",
                        value, src, src
                    )));
                }
            } else {
                let options = environment.people.iter().map(|s| s.to_string()).collect();
                Select::new("Who did you receive this money from?", options).prompt()?
            };
            if value < Decimal::ZERO {
                return Err(KakeboError::InvalidArgument(format!(
                    "Cannot receive {} from {}, {} is negative",
                    value, source_person, value
                )));
            }
            let mut this_value = value;
            let mut overflow = *expenses
                .overflows
                .get(&source_person)
                .unwrap_or(&Decimal::ZERO);
            let mut any_change = false;
            while this_value + overflow > Decimal::ZERO {
                // TODO: implement non-group expense behaviour
                let options: Vec<_> = expenses
                    .group_expenses
                    .iter_mut()
                    .flat_map(|group_expense| {
                        group_expense.parts().filter(|part| {
                            part.person == source_person
                                && part.to_pay > part.paid.unwrap_or(Decimal::ZERO)
                        })
                    })
                    .collect();
                println!(
                    "There is {} + {} = {} unassigned",
                    this_value,
                    overflow,
                    this_value + overflow
                );
                if options.is_empty() {
                    break;
                }
                let pay_off_option =
                    Select::new("Which group expense did this pay?", options).prompt();
                if let Err(inquire::InquireError::OperationCanceled) = pay_off_option {
                    break;
                }
                let part = pay_off_option?;
                let mut possible_targets = expenses
                    .group_expenses
                    .iter_mut()
                    .filter(|group_expense| group_expense.info == part.info);
                let group_expense: &mut GroupExpense = possible_targets
                    .next()
                    .expect("We previously checked for this existence");
                assert!(
                    possible_targets.next().is_none(),
                    "Our implementation is invalid if ExpenseInfo is not unique"
                );

                let mut still_to_pay = part.to_pay - part.paid.unwrap_or(Decimal::ZERO);
                let mut paying = Decimal::ZERO;

                let non_overflow_part = this_value.min(still_to_pay);
                this_value -= non_overflow_part;
                paying += non_overflow_part;
                still_to_pay -= non_overflow_part;

                if !still_to_pay.is_zero() {
                    let overflow_part = overflow.min(still_to_pay);
                    overflow -= overflow_part;
                    paying += overflow_part;
                    still_to_pay -= overflow_part;
                }

                let now_paid = part.paid.unwrap_or(Decimal::ZERO) + paying;
                group_expense.paid_amounts[part.index] = Some(now_paid);
                any_change = true;
            }
            let new_overflow = this_value + overflow;
            if new_overflow
                != *expenses
                    .overflows
                    .get(&source_person)
                    .unwrap_or(&Decimal::ZERO)
            {
                expenses.overflows.insert(source_person, new_overflow);
                any_change = true;
            }
            any_change
        }
        Command::Sanitize => {
            for group_expense in expenses.group_expenses.iter_mut() {
                for (i, to_pay) in group_expense.true_amounts().into_iter().enumerate() {
                    let person = group_expense.people[i].clone();
                    let paid = group_expense.paid_amounts[i].unwrap_or(Decimal::ZERO);
                    if paid > to_pay {
                        group_expense.paid_amounts[i] = Some(to_pay);
                        let overflow = paid - to_pay;
                        expenses
                            .overflows
                            .entry(person)
                            .and_modify(|mut v| v += overflow)
                            .or_insert(overflow);
                    }
                }
            }
            true
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
