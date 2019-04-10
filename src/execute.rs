use std::collections::BTreeMap as Map;
use std::fs;
use std::io::Error;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use ansi_term::Colour::{Green, Yellow};

use crate::commands::{Cmd, FileConfig, Mod};

const PATH_STR: &str = ".donkey-make.tmp";
const BAR: &str = "==========================================================================================";

pub fn main(command_name: &str, config: &FileConfig, cmd: &Cmd, cli_args: &[String], delete_tmp: bool) -> Option<i32> {
    let mut args: Vec<String> = vec![PATH_STR.to_string()];
    args.extend(cmd.args.iter().cloned());
    args.extend(cli_args.iter().cloned());

    let mut env: StrMap = Map::new();
    merge_maps(&mut env, &config.env);
    merge_maps(&mut env, &cmd.env);

    write(command_name, cmd, &args, &env);
    match run_command(command_name, cmd, &args, &env) {
        Ok(t) => {
            delete(delete_tmp);
            t
        }
        Err(e) => {
            delete(delete_tmp);
            exit!(
                "failed to execute command \"{} {}\": {}",
                cmd.executable,
                args.join(" "),
                e
            );
        }
    }
}

fn write(command_name: &str, cmd: &Cmd, args: &[String], env: &StrMap) {
    let path = Path::new(PATH_STR);
    if path.exists() {
        exit!(
            "Error writing temporary file:\n  {} already exists, donkey-make may be running already",
            PATH_STR
        );
    }

    let prefix: Vec<String> = vec![
        BAR.to_string(),
        format!(
            "This is a temporary file generated by donkey-make to execute the command: \"{}\"",
            command_name
        ),
        format!("Command to be executed: \"{} {}\"", cmd.executable, args.join(" ")),
        format!("Environment variables set: {:?}", env),
        "This file should only exist very temporarily while it's be executed.".to_string(),
        BAR.to_string(),
    ];
    let comment = if cmd.executable.starts_with("node") { "//" } else { "#" };
    let sep = format!("\n{} ", comment);

    let mut smart_script: Vec<String> = vec![];
    let script: &Vec<String> = match cmd.modifier {
        Mod::SmartBash => {
            build_smart_script(&mut smart_script, &cmd.run);
            &smart_script
        }
        _ => &cmd.run,
    };

    let content = format!("{} {}\n{}", comment, prefix.join(&sep), script.join("\n"));

    match create_file(path, &content) {
        Ok(t) => t,
        Err(e) => {
            exit!("Error writing temporary file {}:\n  {}", PATH_STR, e);
        }
    };
}

fn run_command(command_name: &str, cmd: &Cmd, args: &[String], env: &StrMap) -> Result<Option<i32>, Error> {
    let mut c = Command::new(&cmd.executable);
    c.args(args).envs(env);
    let sig = register_signals()?;

    let tic = SystemTime::now();
    let status = c.status()?;
    let toc = SystemTime::now();
    let dur_str = format_duration(tic, toc);
    if status.success() {
        printlnc!(Green, "Command \"{}\" successful, took {}", command_name, dur_str);
        Ok(None)
    } else {
        match status.code() {
            Some(c) => {
                printlnc!(
                    Yellow,
                    "Command \"{}\" failed, took {}, exit code {}",
                    command_name,
                    dur_str,
                    c
                );
                Ok(Some(c))
            }
            None => {
                printlnc!(
                    Yellow,
                    "Command \"{}\" kill with signal {} after {}",
                    command_name,
                    signal_name(sig),
                    dur_str
                );
                Ok(Some(2))
            }
        }
    }
}

fn delete(delete: bool) {
    if delete {
        let path = Path::new(PATH_STR);
        match fs::remove_file(path) {
            Ok(t) => t,
            Err(e) => {
                exit!("Error deleting temporary file {}, {}", PATH_STR, e);
            }
        };
    }
}

fn create_file(path: &Path, content: &str) -> std::io::Result<()> {
    let mut f = fs::File::create(path)?;
    f.write_all(content.as_bytes())?;
    Ok(())
}

struct Signal {
    int: Arc<AtomicBool>,
    term: Arc<AtomicBool>,
}

fn register_signals() -> Result<Signal, Error> {
    let sig = Signal {
        int: Arc::new(AtomicBool::new(false)),
        term: Arc::new(AtomicBool::new(false)),
    };
    // TODO this doesn't forward the signal to the child, but generally the terminal does that for us
    signal_hook::flag::register(signal_hook::SIGINT, Arc::clone(&sig.int))?;
    signal_hook::flag::register(signal_hook::SIGTERM, Arc::clone(&sig.term))?;
    Ok(sig)
}

fn signal_name(sig: Signal) -> &'static str {
    if sig.int.load(Ordering::Relaxed) {
        "SIGINT"
    } else if sig.term.load(Ordering::Relaxed) {
        "SIGTERM"
    } else {
        "UNKNOWN"
    }
}

fn build_smart_script(new_script: &mut Vec<String>, original: &[String]) {
    // https://www.gnu.org/software/bash/manual/html_node/The-Set-Builtin.html
    new_script.push("set -ex".to_string());
    if original.len() == 1 {
        if let Some(line) = original.first() {
            if !line.contains('$') {
                new_script.push(format!("{} $@", line));
                return;
            }
        }
    }
    new_script.extend(original.iter().cloned());
}

type StrMap = Map<String, String>;

fn merge_maps(base: &mut StrMap, update: &StrMap) {
    base.extend(update.iter().map(|(k, v)| (k.clone(), v.clone())));
}

fn format_duration(tic: SystemTime, toc: SystemTime) -> String {
    match toc.duration_since(tic).unwrap() {
        d if d < Duration::from_millis(10) => format!("{:0.3}ms", d.subsec_micros() as f32 / 1000.0),
        d if d < Duration::from_secs(1) => format!("{}ms", d.subsec_millis()),
        d if d < Duration::from_secs(100) => {
            format!("{:0.3}s", d.as_secs() as f64 + f64::from(d.subsec_millis()) / 1000.0)
        }
        d => format!("{}s", d.as_secs()),
    }
}
