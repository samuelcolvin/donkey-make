extern crate serde_yaml;
#[macro_use]
extern crate serde_derive;

use std::process;

#[macro_use]
mod macros;

mod cli;
mod commands_file;
mod tmp_file;

fn main() {
    let args = cli::parse();

    let file_data = commands_file::load();

    let command_name = args.value_of("command").unwrap();
    let command = match file_data.get(command_name) {
        Some(c) => c,
        None => {
            exit!(r#"Command {:?} not found"#, command_name);
        }
    };

    tmp_file::write(command_name, command);
    println!(r#"Runnign command "{}"..."#, command_name);
    match tmp_file::run_command(command_name) {
        Some(exit_code) => {
            process::exit(exit_code);
        },
        None => {},
    };
}
