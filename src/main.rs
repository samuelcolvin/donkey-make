extern crate serde_yaml;
#[macro_use]
extern crate serde_derive;
extern crate clap;

use std::process;

#[macro_use]
mod macros;

mod commands;
mod execute;
use crate::commands::FileConfig;

fn main() {
    let cli = parse_args();
    let file_path = commands::find_file(&cli.file_path);

    let config = commands::load_file(file_path);
    let keys: Vec<String> = config.commands.keys().cloned().collect();

    let command_name = get_command(&cli.command, &config, &keys);
    let command = match config.commands.get(&command_name) {
        Some(c) => c,
        None => {
            exit!(
                "Command \"{}\" not found, commands available are:\n  {}",
                command_name,
                keys.join(", ")
            );
        }
    };

    println!(
        r#"Running command "{}" from "{}"..."#,
        command_name,
        file_path.display()
    );

    match execute::main(&command_name, &config, &command, &cli.args) {
        Some(c) => {
            process::exit(c);
        }
        None => {}
    };
}

#[derive(Debug)]
pub struct CliArgs {
    pub file_path: Option<String>,
    pub command: Option<String>,
    pub args: Vec<String>,
}

fn parse_args() -> CliArgs {
    let raw_args = clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            clap::Arg::with_name("file")
                .short("f")
                .long("file")
                .help("File to find commands in")
                .takes_value(true),
        )
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
        .get_matches();

    let mut file_path: Option<String> = None;
    let mut command: Option<String> = None;
    let mut args: Vec<String> = match raw_args.values_of("args") {
        Some(a) => a.map(|v| v.to_string()).collect(),
        None => Vec::new(),
    };

    if let Some(cc_) = raw_args.value_of("command") {
        if cc_.starts_with("./") {
            // special case that donkey-make was used in the shebang line, and the first argument
            // (aka command) is actually the path to the file
            file_path = Some(cc_.to_string());
            if args.len() > 0 {
                command = Some(args.remove(0));
            }
        } else {
            command = Some(cc_.to_string());
        }
    }

    if let Some(cli_file_) = raw_args.value_of("file") {
        file_path = Some(cli_file_.to_string())
    }

    return CliArgs {
        file_path,
        command,
        args,
    };
}

fn get_command(cli_command: &Option<String>, config: &FileConfig, keys: &Vec<String>) -> String {
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
