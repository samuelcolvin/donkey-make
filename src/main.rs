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
        .get_matches()
}

fn main() {
    let args = parse_args();

    let config = commands::load_file();
//    println!("{:?}", config);
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
                }
            }
        }
    };
//
    let command = match config.commands.get(&command_name) {
        Some(c) => c,
        None => {
            exit!("Command \"{}\" not found, options are:\n  {}", command_name, keys.join(", "));
        }
    };

    match execute::main(&command_name, &config, &command) {
        Some(c) => {
            process::exit(c);
        },
        None => {}
    };
}
