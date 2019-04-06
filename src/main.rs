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
                .required(true)
                .index(1),
        )
        .get_matches()
}

fn main() {
    let args = parse_args();

    let commands_data = commands::load_file();

    let command_name = args.value_of("command").unwrap();
    let command = match commands_data.get(command_name) {
        Some(c) => c,
        None => {
            exit!("Command \"{}\" not found", command_name);
        }
    };

    execute::main(command_name, command);
}
