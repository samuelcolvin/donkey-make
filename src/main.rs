extern crate ansi_term;
extern crate atty;
#[macro_use]
extern crate clap;
extern crate linked_hash_map;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
extern crate signal_hook;

#[macro_use]
mod macros;

use std::env;
use std::path::Path;
use std::string::ToString;

use ansi_term::Colour::{Cyan, Green, Red};

use crate::commands::{Cmd, FileConfig};
use crate::utils::{CliArgs, DONKEY_KEEP_ENV};

mod commands;
mod execute;
mod prepare;
mod utils;

fn main() {
    let exit_code = match run() {
        Err(e) => {
            eprintlnc!(Red, "{}", e);
            // use 100 to hopefully differentiate from command error codes
            100
        }
        Ok(c) => c,
    };
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
}

fn run() -> Result<i32, String> {
    let cli = parse_args();
    let file_path = commands::find_file(&cli.file_path)?;

    let config = commands::load_file(&file_path)?;

    let command_name = match &cli.command {
        Some(c) => c,
        _ => {
            help_message(&file_path, &config);
            return Ok(0);
        }
    };
    let cmd = get_command(&config, &command_name)?;

    let run = prepare::main(&command_name, &config, &cmd, &cli, &file_path)?;
    let c = execute::main(&run, &cmd, &cli)?;
    Ok(c)
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

    let keep_tmp = if raw_args.is_present("keep_tmp") {
        true
    } else {
        match env::var(DONKEY_KEEP_ENV) {
            Ok(t) => t == "1",
            _ => false,
        }
    };

    CliArgs {
        file_path,
        command,
        args,
        keep_tmp,
    }
}

fn get_command<'a>(config: &'a FileConfig, command_name: &str) -> Result<&'a Cmd, String> {
    Ok(match config.commands.get(command_name) {
        Some(c) => c,
        None => {
            let keys = config.keys();
            return err!(
                "Command \"{}\" not found, commands available are:\n  {}{}",
                command_name,
                keys.join(", "),
                suggestion(command_name, &keys)
            );
        }
    })
}

fn suggestion(v: &str, possibilities: &[String]) -> String {
    let mut threshold: f64 = 0.8;
    let mut candidate: Option<&String> = None;
    for pv in possibilities {
        let confidence = strsim::jaro_winkler(v, &pv);
        if confidence > threshold {
            threshold = confidence;
            candidate = Some(pv);
        }
    }
    match candidate {
        Some(c) => paint!(Cyan, format!("\n\n    perhaps you meant \"{}\"?", c)),
        _ => "".to_string(),
    }
}

const PAD_TO: usize = 14;

fn summary(key: &str, config: &FileConfig) -> String {
    let cmd = &config.commands[key];
    let pad = match key.chars().count() {
        l if l < PAD_TO => PAD_TO - l,
        _ => 0,
    };
    format!(
        "{}{} {} {}",
        paint!(Cyan, key),
        " ".repeat(pad),
        paint!(Green, cmd.summary()),
        cmd.description()
    )
}

fn help_message(file_path: &Path, config: &FileConfig) {
    let commands: Vec<String> = config.keys().iter().map(|k| summary(k, &config)).collect();
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
