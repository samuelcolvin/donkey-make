use std::collections::BTreeMap as Map;
use std::fs::File;
use std::path::Path;
use std::process;

#[derive(Debug, Serialize, Deserialize)]
pub struct Command {
    name: String,
    age: u8,
    phones: Vec<String>,
}

type FileData = Map<String, Vec<String>>;

pub fn load() -> FileData {
    let path = Path::new("donkey.yaml");

    let file = match File::open(&path) {
        Ok(t) => t,
        Err(e) => {
            exit!("Error opening {}:\n  {}", path.display(), e);
        }
    };

    match serde_yaml::from_reader(file) {
        Ok(t) => t,
        Err(e) => {
            exit!("Error parsing {}:\n  {}", path.display(), e);
        }
    }
}
