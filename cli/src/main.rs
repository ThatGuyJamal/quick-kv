use clap::{arg, Command};

fn cli() -> Command
{
    Command::new("Quick-KV CLI Tool")
        .name("qkv")
        .about("Quick-KV CLI Tool.")
        .long_about(
            r#"Quick-KV CLI Tool.

Quick-KV is a file-based key-value database written in Rust. This CLI tool is used to interact with the database.

"#,
        )
        .version("0.1.0")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(false)
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
}

fn main()
{
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("get", values)) => println!(
            "Getting \"{}\" from the database",
            values.get_one::<String>("KEY").expect("Key not provided?")
        ),
        Some(("set", values)) => println!(
            "Setting \"{}\" to \"{}\" in the database",
            values.get_one::<String>("KEY").expect("Key not provided?"),
            values.get_one::<String>("VALUE").expect("Value not provided?"),
        ),
        Some(("delete", values)) => println!(
            "Deleting \"{}\" from the database",
            values.get_one::<String>("KEY").expect("Key not provided?")
        ),
        Some(("update", values)) => println!(
            "Updating \"{}\" to \"{}\" in the database",
            values.get_one::<String>("KEY").expect("Key not provided?"),
            values.get_one::<String>("VALUE").expect("Value not provided?"),
        ),
        _ => unreachable!(),
    }
}
