#[macro_use]
extern crate serde_derive;
extern crate ansi_term;
#[macro_use]
extern crate clap;
extern crate serde_yaml;

#[macro_use]
mod macros;

use ansi_term::Colour::Green;

use crate::commands::{Cmd, FileConfig};

mod commands;
mod execute;

fn main() {
    let cli = parse_args();
    let file_path = commands::find_file(&cli.file_path);

    let config = commands::load_file(file_path);
    let keys: Vec<String> = config.commands.keys().cloned().collect();

    let command_name = get_command_name(&cli.command, &config, &keys);
    let command = get_command(&config, &command_name, &keys);

    printlnc!(
        Green,
        r#"Running command "{}" from "{}"..."#,
        command_name,
        file_path.display()
    );

    match execute::main(&command_name, &config, &command, &cli.args, cli.delete_tmp) {
        Some(c) => {
            std::process::exit(c);
        }
        None => {}
    };
}

#[derive(Debug)]
pub struct CliArgs {
    pub file_path: Option<String>,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub delete_tmp: bool,
}

fn parse_args() -> CliArgs {
    let cli_yaml = load_yaml!("cli.yaml");
    let mut version = env!("CARGO_PKG_VERSION").to_string();
    if let Some(commit) = option_env!("TRAVIS_COMMIT") {
        version += &format!(" {}", &commit[..7]);
    }

    let raw_args = clap::App::from_yaml(cli_yaml)
        .version(version.as_str())
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
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
        delete_tmp: !raw_args.is_present("dont_delete_tmp"),
    };
}

fn get_command_name(cli_command: &Option<String>, config: &FileConfig, keys: &Vec<String>) -> String {
    if let Some(cli_command_) = cli_command {
        cli_command_.to_string()
    } else if let Some(default_command) = config.default_command.clone() {
        default_command
    } else if let Some(first_command) = keys.first() {
        first_command.to_string()
    } else {
        exit!("no commands found");
    }
}

fn get_command<'a>(config: &'a FileConfig, command_name: &String, keys: &Vec<String>) -> &'a Cmd {
    match config.commands.get(command_name) {
        Some(c) => c,
        None => {
            exit!(
                "Command \"{}\" not found, commands available are:\n  {}",
                command_name,
                keys.join(", ")
            );
        }
    }
}
