use clap::{Parser, Subcommand};
use serde::Deserialize;

use self::errors::KakeboError;
use self::tui::open_widget;
use self::tui::table::{StatefulTable, StatefulTableBuilder, TableData};

pub mod errors;
mod format;
mod parse;
pub mod tui;

fn create_table() -> Result<StatefulTable, KakeboError> {
    Ok(StatefulTableBuilder::new("Table kind".into())
        .table_data(
            TableData::new(
                "User description of table".into(),
                &["Col 1".into(), "Col 2".into()],
                &["Row 1".into(), "Row 2".into()],
                &[1, 2, 3, 4],
            )
            .unwrap(),
        )
        .editable_column(0)
        .build())
}

#[derive(Debug, Deserialize)]
pub struct KakeboConfig {
    pub currency: char,
    pub decimal_sep: char,
    pub user_name: String,
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
    Test, // HACK: Remove this
}

#[derive(Subcommand, Debug)]
enum ExpenseType {
    Single,
    Group,
    Recurring,
}

// fn parse_config() -> Result<KakeboConfig, KakeboError> {
//     let cur_dir = std::env::current_dir()?;
//     let config_path = cur_dir.join("kakebo.config");

//     if !config_path.exists() {
//         println!("No config file found at {}", config_path.display());
//         println!(
//             "Please create a config file. A minimal config would look like this:
//     \"user_name\" = \"Your name\"
//     \"currency\" = \"$\"
//     \"decimal_sep\" = \".\""
//         );
//         return Err(KakeboError::InvalidArgument("No config file found".into()));
//     }

//     println!("Config file found at {}", config_path.display());
//     let config = std::fs::read_to_string(config_path)?;
//     let config: KakeboConfig = toml::from_str(&config)?;
//     Ok(config)
// }

fn main() -> Result<(), KakeboError> {
    let args = Args::parse();

    match args.command {
        Command::Status => println!("Status"),
        Command::Add { expense_type } => match expense_type {
            ExpenseType::Single => println!("Add single"),
            ExpenseType::Group => println!("Add group"),
            ExpenseType::Recurring => println!("Add recurring"),
        },
        Command::Edit { expense_type } => match expense_type {
            ExpenseType::Single => println!("Edit single"),
            ExpenseType::Group => println!("Edit group"),
            ExpenseType::Recurring => println!("Edit recurring"),
        },
        Command::Receive { value, from } => println!("Receive {} from {}", value, from),
        Command::Test => open_widget(create_table()?)?,
    }
    Ok(())
}
