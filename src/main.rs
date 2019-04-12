#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
extern crate ansi_term;
#[macro_use]
extern crate clap;
extern crate signal_hook;
extern crate indexmap;

#[macro_use]
mod macros;

use std::path::Path;
use std::string::ToString;

use ansi_term::Colour::{Cyan, Green, Red, Yellow};
use ansi_term::Style;

use crate::commands::{Cmd, FileConfig};

mod commands;
mod execute;

fn main() {
    let optional_exit_code = match run() {
        Err(e) => {
            eprintln!("{}", Red.paint(e));
            // use 100 to hopefully differentiate from command error codes
            Some(100)
        }
        Ok(c) => c,
    };
    if let Some(exit_code) = optional_exit_code {
        std::process::exit(exit_code);
    }
}

fn run() -> Result<Option<i32>, String> {
    let cli = parse_args();
    let file_path = commands::find_file(&cli.file_path)?;

    let config = commands::load_file(file_path)?;
    let keys: Vec<String> = config.commands.keys().cloned().collect();

    let command_name = match cli.command {
        Some(c) => c,
        _ => {
            help_message(&file_path, &config, &keys);
            return Ok(None);
        }
    };
    let command = get_command(&config, &command_name, &keys)?;

    printlnc!(
        Green,
        r#"Running command "{}" from {}..."#,
        command_name,
        file_path.display()
    );

    let c = execute::main(&command_name, &config, &command, &cli.args, cli.delete_tmp)?;
    Ok(c)
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
    let mut version = get_version();
    if let Some(commit) = option_env!("TRAVIS_COMMIT") {
        version += &format!(" {}", &commit[..7]);
    }
    let raw_args = clap::App::from_yaml(cli_yaml)
        .version(version.as_str())
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(include_str!("about.txt"))
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

fn get_command<'a>(config: &'a FileConfig, command_name: &str, keys: &[String]) -> Result<&'a Cmd, String> {
    Ok(match config.commands.get(command_name) {
        Some(c) => c,
        None => {
            return err!(
                "Command \"{}\" not found, commands available are:\n  {}",
                command_name,
                keys.join(", ")
            );
        }
    })
}

fn summary(key: &str, config: &FileConfig) -> String {
    let cmd = &config.commands[key];
    let description = format!("- {}", &cmd.description);
    format!(
        "{} {}",
        Style::new().fg(Cyan).paint(key),
        Style::new().fg(Yellow).paint(description),
    )
}

fn help_message(file_path: &Path, config: &FileConfig, keys: &[String]) {
    let commands: Vec<String> = keys.iter().map(|k| summary(k, &config)).collect();
    printlnc!(
        Green,
        "donkey-make {}, commands available from {}:\n  {}",
        get_version(),
        file_path.display(),
        commands.join("\n  ")
    );
}

fn get_version() -> String {
    format!("v{}", env!("CARGO_PKG_VERSION"))
}
