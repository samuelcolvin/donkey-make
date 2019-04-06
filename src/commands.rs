use std::collections::BTreeMap as Map;
use std::fs::File;
use std::path::Path;
use std::process;

use serde_yaml::{Value, from_value};
use serde::de::{Error, Deserialize, Deserializer};

#[derive(Debug)]
pub struct Cmd {
    pub run: Vec<String>,
    pub args: Vec<String>,
    pub env: Map<String, String>,
    pub executable: String,
    // TODO context, before
}

pub fn load_file() -> Map<String, Cmd> {
    let path = Path::new("donkey-make.yaml");

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

fn dft_exe() -> String {
    "bash".to_string()
}

#[derive(Debug, Deserialize)]
struct Command {
    run: Vec<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: Map<String, String>,
    #[serde(default = "dft_exe")]
    executable: String,
}

impl<'de> Deserialize<'de> for Cmd {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v: Value = Deserialize::deserialize(deserializer)?;
        if v.is_sequence() {
            let run: Vec<String> = from_value(v).map_err(D::Error::custom)?;
            Ok(Cmd {
                run,
                args: Vec::new(),
                env: Map::new(),
                executable: dft_exe()
            })
        } else {
            let c: Command = from_value(v).map_err(D::Error::custom)?;
            Ok(Cmd {
                run: c.run,
                args: c.args,
                env: c.env,
                executable: c.executable,
            })
        }
    }
}
