use std::env;
use std::path::{Path, PathBuf};

use crate::commands;

pub const COMPLETION_SCRIPT: &str = "--completion-script";
pub const COMPLETE_COMMAND: &str = "--complete-command";

#[allow(unused_must_use)]
pub fn main() -> bool {
    let args: Vec<String> = env::args().collect();
    if args.contains(&COMPLETION_SCRIPT.to_string()) {
        print!("{}", include_str!("completion.sh"));
    } else if args.contains(&COMPLETE_COMMAND.to_string()) {
        // errors in complete_command are ignored, we just don't make any suggestions
        complete_command(args);
    } else {
        return false;
    }
    true
}

fn complete_command(args: Vec<String>) -> Result<(), String> {
    let config = if args.iter().count() == 3 && Path::new(&args[2]).is_file() {
        let file_path = PathBuf::from(args[2].clone());
        match commands::load_file(&file_path) {
            Err(_) => default_config()?,
            Ok(c) => c,
        }
    } else {
        default_config()?
    };

    println!("{}", config.keys().join(" "));
    Ok(())
}

fn default_config() -> Result<commands::FileConfig, String> {
    let file_path = commands::find_file(&None)?;
    commands::load_file(&file_path)
}
