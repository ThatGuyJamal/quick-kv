use std::io::{self, Write};

use clap::{arg, Command};
use log::LevelFilter;
use quick_kv::prelude::*;

const START_MESSAGE: &str = r#"
Welcome to the Quick-KV REPL!

Run 'qkv help' to see the list of commands.
"#;

fn cli() -> Command
{
    Command::new("Quick-KV REPL")
        .name("qkv")
        .about("REPL for interacting with Quick-KV")
        .long_about(START_MESSAGE)
        .subcommand_required(true)
        .arg_required_else_help(false)
        .allow_external_subcommands(false)
        .subcommand(Command::new("version").about("Prints the version of the CLI tool"))
        .subcommand(
            Command::new("get")
                .about("Gets data from the database")
                .arg(arg!(<KEY> "Key to get the value of"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("set")
                .about("Sets new data into the database")
                .arg(arg!(<KEY> "Key to set the value of"))
                .arg(arg!(<VALUE> "Value to set"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("delete")
                .about("Deletes data from the database")
                .arg(arg!(<KEY> "Key to delete"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("update")
                .about("Update data in the database")
                .arg(arg!(<KEY> "Key to update the value of"))
                .arg(arg!(<VALUE> "New value to set for the key"))
                .arg_required_else_help(true),
        )
        .subcommand(Command::new("keys").about("Lists all keys in the database"))
        .subcommand(Command::new("exit").about("Exits the repl"))
    // .subcommand(Command::new("").about(""))
}

// todo - fix bug where if you type incorrect commands then the repl crashes.
// todo - make a config file where users can define the type of data the database will store.
fn main() -> anyhow::Result<()>
{
    let client = QuickClient::<String>::new(ClientConfig::new(
        "cli.qkv".to_string(),
        true.into(),
        LevelFilter::Debug.into(),
    ));

    println!("{}", START_MESSAGE);

    loop {
        print!(">> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input = input.trim();

        if !input.starts_with("qkv") {
            println!("Input must start with 'qkv'. Type 'qkv help' for more information.");
            continue;
        }

        let mut command_recognized = false;

        match input {
            "exit" => {
                println!("Exiting repl...");
                break;
            }
            _ => {
                let matches = cli().get_matches_from(input.split_whitespace().collect::<Vec<_>>());

                match matches.subcommand() {
                    Some(("version", _)) => {
                        println!("Quick-KV CLI v{}", env!("CARGO_PKG_VERSION"));
                        command_recognized = true;
                    }
                    Some(("get", args)) => {
                        let key: &String = args.get_one::<String>("KEY").expect("Key not provided?");
                        get(client.clone(), key)?;
                        command_recognized = true;
                    }
                    Some(("set", args)) => {
                        let key = args.get_one::<String>("KEY").expect("Key not provided?");
                        let value = args.get_one::<String>("VALUE").expect("Value not provided?");
                        set(client.clone(), key, value.to_string())?;
                        command_recognized = true;
                    }
                    Some(("delete", args)) => {
                        let key = args.get_one::<String>("KEY").expect("Key not provided?");
                        delete(client.clone(), key)?;
                        command_recognized = true;
                    }
                    Some(("update", args)) => {
                        let key = args.get_one::<String>("KEY").expect("Key not provided?");
                        let value = args.get_one::<String>("VALUE").expect("Value not provided?");
                        update(client.clone(), key, value.to_string())?;
                        command_recognized = true;
                    }
                    Some(("keys", _)) => {
                        keys(client.clone())?;
                        command_recognized = true;
                    }
                    _ => println!("Unknown command. Type 'exit' to quit."),
                }

                if !command_recognized {
                    println!("Invalid command. Type 'exit' to quit.");
                }
            }
        }
    }

    Ok(())
}

fn get(mut client: QuickClient<String>, key: &str) -> anyhow::Result<()>
{
    let result = client.get(key)?;

    if let Some(value) = result {
        println!("\"{}\"", value);
    } else {
        println!("No value found for \"{}\"", key);
    }

    Ok(())
}

fn set(mut client: QuickClient<String>, key: &str, value: String) -> anyhow::Result<()>
{
    client.set(key, value.clone())?;

    std::thread::sleep(std::time::Duration::from_secs(5));

    println!("set: \"{}\"", key);
    Ok(())
}

fn update(mut client: QuickClient<String>, key: &str, value: String) -> anyhow::Result<()>
{
    client.update(key, value.to_owned(), None)?;

    println!("Updated \"{}\"", key);
    Ok(())
}

fn delete(mut client: QuickClient<String>, key: &str) -> anyhow::Result<()>
{
    client.delete(key)?;

    println!("Deleted \"{}\"", key);
    Ok(())
}

fn keys(mut client: QuickClient<String>) -> anyhow::Result<()>
{
    let keys = client.keys()?;

    println!("Keys: {:?}", keys);
    Ok(())
}
