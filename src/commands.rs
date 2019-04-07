use std::collections::BTreeMap as Map;
use std::fs::File;
use std::path::Path;
use std::process;

use serde::de::{Deserialize, Deserializer, Error};
use serde_yaml::{from_value, Value};

#[derive(Debug, Deserialize)]
pub struct FileConfig {
    #[serde(rename = ".env")]
    #[serde(default)]
    pub env: Map<String, String>,

    #[serde(rename = ".default")]
    #[serde(default)]
    pub default_command: Option<String>,

    #[serde(flatten)]
    pub commands: Map<String, Cmd>,
    // TODO context
}

#[derive(Debug)]
pub struct Cmd {
    pub run: Vec<String>,
    pub args: Vec<String>,
    pub env: Map<String, String>,
    pub executable: String,
    // TODO context, before
}

const PATH_OPTIONS: [&'static str; 6] = [
    "donkey-make.yaml",
    "donkey-make.yml",
    "donkey.yaml",
    "donkey.yml",
    "donk.yaml",
    "donk.yml",
];

pub fn find_file(file_path_opt: &Option<String>) -> &Path {
    if let Some(file_path) = file_path_opt {
        return Path::new(file_path);
    }
    let mut path_option: &Path;
    for path in PATH_OPTIONS.iter() {
        path_option = Path::new(path);
        if path_option.exists() {
            return path_option;
        }
    }
    exit!("No commands file provided, and no default found, tried:\n  donkey-make.ya?ml, donkey.ya?ml and donk.ya?ml");
}

pub fn load_file(path: &Path) -> FileConfig {
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

// Command here is copy of Cmd above, used for deserialising maps
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
                executable: dft_exe(),
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
