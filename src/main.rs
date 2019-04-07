extern crate serde_yaml;
#[macro_use]
extern crate serde_derive;
extern crate clap;

use std::process;

#[macro_use]
mod macros;

mod commands;
mod execute;

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

fn main() {
    let args = parse_args();

    let config = commands::load_file();
    let keys: Vec<String> = config.commands.keys().cloned().collect();

    let command_name = match args.value_of("command") {
        Some(c) => c.to_string(),
        None => {
            let default_command = config.default_command.clone();
            match default_command {
                Some(c) => c.clone().to_string(),
                None => match keys.first() {
                    Some(c) => c.to_string(),
                    None => {
                        exit!("no commands found");
                    }
                },
            }
        }
    };
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
    let cli_args: Vec<String> = match args.values_of("args") {
        Some(a) => a.map(|v| v.to_string()).collect(),
        None => Vec::new(),
    };

    match execute::main(&command_name, &config, &command, &cli_args) {
        Some(c) => {
            process::exit(c);
        }
        None => {}
    };
}
