use std::path::Path;
use std::env;

use crate::commands;

pub const COMPLETION_SCRIPT: &str = "--completion-script";
pub const COMPLETE_COMMAND: &str = "--complete-command";

#[allow(unused_must_use)]
pub fn main() -> bool {
    let args: Vec<String> = env::args().collect();
    if args.contains(&COMPLETION_SCRIPT.to_string()) {
        print!("{}", include_str!("completion.sh"));;
    } else if args.contains(&COMPLETE_COMMAND.to_string()) {
        // errors in complete_command are ignored, we just don't make any suggestions
        complete_command(args);
    } else {
        return false;
    }
    return true;
}

fn complete_command(args: Vec<String>) -> Result<(), String> {
    let file_path = if args.iter().count() == 3 {
        let path_str = args[2].clone();
        if Path::new(&path_str).exists() {
            Some(path_str)
        } else {
            None
        }
    } else {
        None
    };

    let file_path = commands::find_file(&file_path)?;
    let config = commands::load_file(&file_path)?;

    println!("{}", config.keys().join(" "));
    Ok(())
}
