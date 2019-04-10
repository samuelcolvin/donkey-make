#[macro_use]
extern crate serde_derive;
extern crate ansi_term;
#[macro_use]
extern crate clap;
extern crate serde_yaml;
extern crate signal_hook;

#[macro_use]
mod macros;

use std::path::Path;
use std::string::ToString;

use ansi_term::Colour::Green;

use crate::commands::{Cmd, FileConfig};

mod commands;
mod execute;

fn main() {
    let cli = parse_args();
    let file_path = commands::find_file(&cli.file_path);

    let config = commands::load_file(file_path);
    let keys: Vec<String> = config.commands.keys().cloned().collect();

    let command_name = match cli.command {
        Some(c) => c,
        _ => {
            help_message(&file_path, &keys);
            return;
        }
    };
    let command = get_command(&config, &command_name, &keys);

    printlnc!(
        Green,
        r#"Running command "{}" from {}..."#,
        command_name,
        file_path.display()
    );

    if let Some(c) = execute::main(&command_name, &config, &command, &cli.args, cli.delete_tmp) {
        std::process::exit(c);
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
    let raw_args = clap::App::from_yaml(cli_yaml)
        .version(get_version().as_str())
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .get_matches();

    let mut file_path: Option<String> = None;
    let mut command: Option<String> = None;
    let mut args: Vec<String> = match raw_args.values_of("args") {
        Some(a) => a.map(ToString::to_string).collect(),
        None => Vec::new(),
    };

    if let Some(cc_) = raw_args.value_of("command") {
        if cc_.starts_with("./") {
            // special case that donkey-make was used in the shebang line, and the first argument
            // (aka command) is actually the path to the file
            file_path = Some(cc_.to_string());
            if !args.is_empty() {
                command = Some(args.remove(0));
            }
        } else {
            command = Some(cc_.to_string());
        }
    }

    if let Some(cli_file_) = raw_args.value_of("file") {
        file_path = Some(cli_file_.to_string())
    }

    CliArgs {
        file_path,
        command,
        args,
        delete_tmp: !raw_args.is_present("dont_delete_tmp"),
    }
}

fn get_command<'a>(config: &'a FileConfig, command_name: &str, keys: &[String]) -> &'a Cmd {
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

fn help_message(file_path: &Path, keys: &[String]) {
    printlnc!(
        Green,
        "donkey-make {}\nCommands available from {}:\n  {}",
        get_version(),
        file_path.display(),
        keys.join("\n  ") // TODO prettier with description and colour
    );
}

fn get_version() -> String {
    let mut version = env!("CARGO_PKG_VERSION").to_string();
    if let Some(commit) = option_env!("TRAVIS_COMMIT") {
        version += &format!(" {}", &commit[..7]);
    }
    version
}
