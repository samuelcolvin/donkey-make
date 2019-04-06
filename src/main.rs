extern crate serde_yaml;

#[macro_use]
extern crate serde_derive;

use std::process;
use std::fs::File;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
struct Person {
    name: String,
    age: u8,
    phones: Vec<String>,
}

fn load_file() -> Result<Person, String> {
    let path = Path::new("donkey2.yaml");

    let file = match File::open(&path) {
        Ok(t) => t,
        Err(why) => {
            return Err(format!("couldn't open {}, {}", path.display(), why));
        }
    };

    let p: Person = match serde_yaml::from_reader(file) {
        Ok(t) => t,
        Err(why) => return Err(format!("YAML error: {}", why)),
    };
    Ok(p)
}

fn main() {
    let p = match load_file() {
        Ok(t) => t,
        Err(s) => {
            println!("Error Loading file: {}", s);
            process::exit(1);
        }
    };

    println!("{:?}", p);
    //    error!("testing: {}, {}", p.name, p.name);
}
