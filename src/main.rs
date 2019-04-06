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

    let command_name = args.value_of("command").expect("Unexpected Error: command missing");
    let command = match file_data.get(command_name) {
        Some(c) => c,
        None => {
            exit!(r#"Command {:?} not found"#, command_name);
        }
    };

    tmp_file::write(command_name, command);
    tmp_file::delete();
}
