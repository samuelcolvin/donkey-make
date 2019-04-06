extern crate serde_yaml;

#[macro_use]
extern crate serde_derive;

use std::fs::File;
use std::path::Path;
use std::process;

#[derive(Debug, Serialize, Deserialize)]
struct Person {
    name: String,
    age: u8,
    phones: Vec<String>,
}

macro_rules! err {
    ($msg:expr) => (
        Err($msg)
    );
    ($fmt:expr, $($arg:expr),+) => (
        Err(format!($fmt, $($arg),+))
    );
}

fn load_file() -> Result<Person, String> {
    let path = Path::new("donkey.yaml");

    let file = match File::open(&path) {
        Ok(t) => t,
        Err(e) => return err!("couldn't open {}, {}", path.display(), e),
    };

    let p: Person = match serde_yaml::from_reader(file) {
        Ok(t) => t,
        Err(e) => return err!("YAML error: {}", e),
    };
    Ok(p)
}

fn main() {
    let p = match load_file() {
        Ok(t) => t,
        Err(s) => {
            eprintln!("Error Loading file:\n  {}", s);
            process::exit(1);
        }
    };

    println!("{:?}", p);
    //    error!("testing: {}, {}", p.name, p.name);
}
