use clap::{Parser, Subcommand};
use expense_editor::{ExpenseEditor, SingleExpense};
use serde::Deserialize;

use self::errors::KakeboError;

pub mod errors;
mod expense_editor;
mod format;
mod parse;

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

fn main() -> Result<(), KakeboError> {
    let args = Args::parse();
    let config = parse_config()?;

    match args.command {
        Command::Status => println!("Status"),
        Command::Add { expense_type } => {
            let mut editor = ExpenseEditor::<SingleExpense>::new(expense_type, config);
            println!("{:?}", editor.create_record()?);
        }
        Command::Edit { expense_type } => match expense_type {
            ExpenseType::Single => println!("Edit single"),
            ExpenseType::Group => println!("Edit group"),
            ExpenseType::Recurring => println!("Edit recurring"),
        },
        Command::Receive { value, from } => println!("Receive {} from {}", value, from),
    }
    Ok(())
}
