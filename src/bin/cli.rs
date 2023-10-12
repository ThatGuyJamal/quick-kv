use clap::{arg, Command};
use quick_kv::prelude::*;

fn cli() -> Command
{
    Command::new("Quick-KV REPL")
        .name("qkv")
        .about("REPL for interacting with Quick-KV")
        .long_about(
            r#"
Quick-KV is a file-based key-value database written in Rust. This CLI tool is used to interact with the database.
"#,
        )
        .subcommand_required(true)
        .arg_required_else_help(true)
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
        .subcommand(Command::new("exit").about("Exits the repl"))
    // .subcommand(Command::new("").about(""))
}

fn main() -> anyhow::Result<()>
{
    let mut client = QuickClient::<i32>::new(ClientConfig::default());

    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("version", _)) => println!("Quick-KV CLI v{}", env!("CARGO_PKG_VERSION")),
        Some(("get", values)) => {
            let key = values.get_one::<String>("KEY").expect("Key not provided?");
            let result = client.get(key.as_str())?;
            if let Some(value) = result {
                println!("Value for \"{}\": {}", key, value);
            } else {
                println!("No value found for \"{}\"", key);
            }
        }
        Some(("set", values)) => {
            let key = values.get_one::<String>("KEY").expect("Key not provided?");
            let value = values.get_one::<String>("VALUE").expect("Value not provided?");

            client.set(key.as_str(), value.parse::<i32>()?)?;

            println!("Set \"{}\" to \"{}\"", key, value);
        }
        Some(("delete", values)) => {
            let key = values.get_one::<String>("KEY").expect("Key not provided?");

            client.delete(key.as_str())?;

            println!("Deleted \"{}\"", key);
        }
        Some(("update", values)) => {
            let key = values.get_one::<String>("KEY").expect("Key not provided?");
            let value = values.get_one::<String>("VALUE").expect("Value not provided?");

            client.update(key.as_str(), value.parse::<i32>()?, None)?;

            println!("Updated \"{}\" to \"{}\"", key, value);
        }
        Some(("exit", _)) => unreachable!(), // Exit the loop to end the REPL.
        _ => println!("Unknown command. Type 'exit' to quit."),
    }

    Ok(())
}
