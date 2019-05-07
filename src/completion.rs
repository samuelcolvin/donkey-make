use std::env;
use std::path::Path;

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
    let file_path = if args.iter().count() == 3 && Path::new(&args[2]).is_file() {
        Some(args[2].clone())
    } else {
        None
    };

    let file_path = commands::find_file(&file_path)?;
    let config = commands::load_file(&file_path)?;

    println!("{}", config.keys().join(" "));
    Ok(())
}
