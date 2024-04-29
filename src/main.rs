use age::{secrecy::Secret, Decryptor, Encryptor};
use clap::{Parser, Subcommand};
use rpassword::read_password;
use serde::Deserialize;

use std::fs::File;
use std::io::{stdout, BufReader, Read, Write};

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
    Test,  // HACK: Remove this
    Crypt, // HACK: Remove this
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

fn crypt_test() -> Result<(), KakeboError> {
    print!("Enter a passphrase: ");
    stdout().flush()?;
    let passphrase = read_password()?;

    let file = File::open("test.txt")?;
    let mut file_reader = BufReader::new(file);

    // ... and decrypt the ciphertext to the plaintext again using the same passphrase.
    let decrypted = {
        let decryptor = match Decryptor::new(&mut file_reader)? {
            age::Decryptor::Passphrase(d) => d,
            _ => unreachable!(),
        };

        let mut decrypted = String::new();

        let mut reader = decryptor.decrypt(&Secret::new(passphrase.to_owned()), None)?;
        reader.read_to_string(&mut decrypted)?;

        decrypted
    };

    println!("Decrypted version of the file:\n{}", decrypted);

    // let encrypted = {
    //     let encryptor = Encryptor::with_user_passphrase(Secret::new(passphrase.to_owned()));

    //     let mut encrypted = vec![];
    //     let mut writer = encryptor.wrap_output(&mut encrypted)?;
    //     writer.write_all(plaintext)?;
    //     writer.finish()?;

    //     encrypted
    // };

    // file.write_all(encrypted.as_slice())?;

    // assert_eq!(decrypted, plaintext);

    Ok(())
}

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
        Command::Crypt => crypt_test()?,
    }
    Ok(())
}
