use std::{
    fs::File,
    io::{stdout, BufReader, BufWriter, Write},
    path::Path,
};

use age::{secrecy::Secret, Decryptor, Encryptor};
use clap::{Parser, Subcommand};
use expenses::{group_expense::GroupExpense, single_expense::SingleExpense};
use lz4_flex::frame::{FrameDecoder, FrameEncoder};
use rpassword::read_password;
use serde::{Deserialize, Serialize};

use self::errors::KakeboError;

pub mod errors;
// mod expense_editor;
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
    Add {
        #[command(subcommand)]
        expense_type: ExpenseType,
    },
    Edit {
        #[command(subcommand)]
        expense_type: ExpenseType,
    },
    Receive {
        #[arg(short, long)]
        value: f64,
        #[arg(short, long)]
        from: String,
    },
}

#[derive(Subcommand, Debug)]
enum ExpenseType {
    Single,
    Group,
    Recurring,
}

#[derive(Debug, Serialize, Deserialize)]
struct Expenses {
    single_expenses: Vec<SingleExpense>,
    group_expenses: Vec<GroupExpense>,
}

impl Expenses {
    pub fn new() -> Self {
        Self {
            single_expenses: Vec::new(),
            group_expenses: Vec::new(),
        }
    }
}

const KAKEBO_DB_FILE: &str = "test.kakebo";

fn main() -> Result<(), KakeboError> {
    let args = Args::parse();
    let config = parse_config()?;

    let path = Path::new(KAKEBO_DB_FILE);

    let mut expenses = if path.exists() {
        let file = File::open(path)?;
        let mut file_reader = BufReader::new(file);
        // TODO: find and fix the bug in the following code to make encryption and decompression work
        // print!("Enter decryption password: ");
        // stdout().flush()?;
        // let passphrase = read_password()?;
        // let decryptor = match Decryptor::new(&mut file_reader)? {
        //     Decryptor::Passphrase(decr) => decr,
        //     _ => unreachable!(),
        // };
        // let mut decrypt_reader = decryptor.decrypt(&Secret::new(passphrase.to_owned()), None)?;
        // let mut decode_reader = FrameDecoder::new(decrypt_reader);
        rmp_serde::decode::from_read(&mut file_reader)?
    } else {
        println!("Starting with an empty database");
        Expenses::new()
    };

    if args.debug {
        println!(
            "=== Expenses Before ===\n{:?}\n=======================",
            expenses
        );
    }

    match args.command {
        Command::Status => println!("Status"),
        Command::Add { expense_type } => {
            match expense_type {
                ExpenseType::Single => {
                    let single = SingleExpense::new(&config)?;
                    println!("{:?}", single);
                    expenses.single_expenses.push(single);
                }
                ExpenseType::Group => {
                    let group = GroupExpense::new(&config)?;
                    println!("{:?}", group);
                    println!("Raw Total {:?}", group.raw_total());
                    println!("Raw Total (scaled) {:?}", group.raw_total().scale());
                    println!("True user amount {:?}", group.true_user_amount());
                    println!("True amounts {:?}", group.true_amounts());
                    expenses.group_expenses.push(group);
                }
                ExpenseType::Recurring => println!("Add group"),
            };
        }
        Command::Edit { expense_type } => match expense_type {
            ExpenseType::Single => println!("Edit single"),
            ExpenseType::Group => println!("Edit group"),
            ExpenseType::Recurring => println!("Edit recurring"),
        },
        Command::Receive { value, from } => println!("Receive {} from {}", value, from),
    }

    if args.debug {
        println!(
            "=== Expenses After ===\n{:?}\n======================",
            expenses
        );
    }

    let file = File::create(KAKEBO_DB_FILE)?;
    let mut file_writer = BufWriter::new(file);
    // TODO: find and fix the bug in the following code to make encryption and decompression work
    // print!("Enter encryption password: ");
    // stdout().flush()?;
    // let passphrase = read_password()?;
    // let encryptor = Encryptor::with_user_passphrase(Secret::new(passphrase.to_owned()));
    // let mut encrypt_writer = encryptor.wrap_output(&mut file_writer)?;
    // let mut compress_writer = FrameEncoder::new(encrypt_writer);
    rmp_serde::encode::write_named(&mut file_writer, &expenses)?;

    Ok(())
}
