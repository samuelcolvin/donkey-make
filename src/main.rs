extern crate serde_yaml;
#[macro_use]
extern crate serde_derive;
extern crate clap;

use std::process;

#[macro_use]
mod macros;

mod commands;
mod execute;
use crate::commands::Config;

fn main() {
    let args = parse_args();
    let mut file_path: Option<String> = None;
    let mut cli_command: Option<String> = None;
    let mut cli_args: Vec<String> = match args.values_of("args") {
        Some(a) => a.map(|v| v.to_string()).collect(),
        None => Vec::new(),
    };

    // special case that donkey-make was used in the shebang line, and the first argument
    // (aka command) is actually the path to the file
    if let Some(cc_) = args.value_of("command") {
        if cc_.starts_with("./") {
            file_path = Some(cc_.to_string());
            if cli_args.len() > 0 {
                cli_command = Some(cli_args.remove(0));
            }
        } else {
            cli_command = Some(cc_.to_string());
        }
    }
    // TODO check --file argument

    let config = commands::load_file(&file_path);
    let keys: Vec<String> = config.commands.keys().cloned().collect();

    let command_name= get_command(&cli_command, &config, &keys);
    let command = match config.commands.get(&command_name) {
        Some(c) => c,
        None => {
            exit!(
                "Command \"{}\" not found, options are:\n  {}",
                command_name,
                keys.join(", ")
            );
        }
    };

    match execute::main(&command_name, &config, &command, &cli_args) {
        Some(c) => {
            process::exit(c);
        }
        None => {}
    };
}

fn parse_args() -> clap::ArgMatches<'static> {
    clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            clap::Arg::with_name("command")
                .help("Command to execute")
                .required(false)
                .index(1),
        )
        .arg(
            clap::Arg::with_name("args")
                .multiple(true)
                .required(false)
                .help("Extra arguments to pass to the command"),
        )
        .get_matches()
}

fn get_command(cli_command: &Option<String>, config: &Config, keys: &Vec<String>) -> String {
    if let Some(cli_command_) = cli_command {
        return cli_command_.to_string();
    } else if let Some(default_command) = config.default_command.clone() {
        return default_command;
    } else if let Some(first_command) = keys.first() {
        return first_command.to_string();
    } else {
        exit!("no commands found");
    }
}
