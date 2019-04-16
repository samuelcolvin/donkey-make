use std::fmt;
use std::fs::File;
use std::marker::PhantomData;
use std::path::Path;

use linked_hash_map::LinkedHashMap as Map;
use serde::de::{self, Deserialize, Deserializer, Error, SeqAccess, Visitor};
use serde_yaml::{from_reader, from_value, Mapping, Value};

#[derive(Debug, Deserialize)]
pub struct FileConfig {
    #[serde(rename = ".env")]
    #[serde(default)]
    pub env: Map<String, String>,

    #[serde(flatten)]
    pub commands: Map<String, Cmd>,
    // TODO context
}

const BASH_SMART: &str = "bash-smart";
const BASH: &str = "bash";

#[derive(Debug)]
pub enum Mod {
    None,
    SmartBash,
}

#[derive(Debug)]
pub struct Cmd {
    pub run: Vec<String>,
    pub args: Vec<String>,
    pub env: Map<String, String>,
    pub executable: String,
    pub description: String,
    pub modifier: Mod,
    // TODO context, before
}

impl Cmd {
    fn from_command(command: Command) -> Cmd {
        let mut modifier: Mod = Mod::SmartBash;
        let executable = match command.executable.as_ref() {
            BASH_SMART => BASH.to_string(),
            e => {
                modifier = Mod::None;
                e.to_string()
            }
        };
        let description = build_description(&command, &modifier);
        Cmd {
            run: command.run,
            args: command.args,
            env: command.env,
            executable,
            description,
            modifier,
        }
    }
}

const PATH_OPTIONS: [&str; 6] = [
    "donk.yml",
    "donk.yaml",
    "donkey.yml",
    "donkey.yaml",
    "donkey-make.yml",
    "donkey-make.yaml",
];

pub fn find_file(file_path_opt: &Option<String>) -> Result<&Path, String> {
    if let Some(file_path) = file_path_opt {
        return Ok(Path::new(file_path));
    }
    for path in PATH_OPTIONS.iter() {
        let path_option: &Path = Path::new(path);
        if path_option.exists() {
            return Ok(path_option);
        }
    }
    err!("No commands file provided, and no default found, tried:\n  donk.ya?ml, donkey.ya?ml and donkey-make.ya?ml")
}

pub fn load_file(path: &Path) -> Result<FileConfig, String> {
    let file = match File::open(&path) {
        Ok(t) => t,
        Err(e) => {
            return err!("Error opening {}:\n  {}", path.display(), e);
        }
    };

    Ok(match from_reader(file) {
        Ok(t) => t,
        Err(e) => {
            return err!("Error parsing {}:\n  {}", path.display(), e);
        }
    })
}

fn build_description(command: &Command, modifier: &Mod) -> String {
    let main = match &command.description {
        Some(d) => d.clone(),
        None => first_line(&command.run),
    };
    let ex_str = match modifier {
        Mod::SmartBash => "".to_string(),
        _ => format!("{}, ", command.executable),
    };
    let lines = match command.run.len() {
        1 => "1 line".to_string(),
        c => format!("{} lines", c),
    };
    format!("{} ({}{})", main, ex_str, lines)
}

fn first_line(run: &[String]) -> String {
    match run.first() {
        Some(f) => {
            let mut first_line = f.clone();
            let mut more = match run.len() {
                c if c > 1 => true,
                _ => false,
            };
            if let Some(nl) = first_line.find('\n') {
                more = true;
                first_line = first_line[..nl].to_string();
            }
            if more {
                format!("{} ...", first_line)
            } else {
                first_line
            }
        }
        None => "".to_string(),
    }
}

fn dft_exe() -> String {
    BASH_SMART.to_string()
}

#[derive(Debug, Deserialize)]
struct Command {
    #[serde(deserialize_with = "string_or_seq")]
    run: Vec<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: Map<String, String>,
    #[serde(rename = "ex")]
    #[serde(default = "dft_exe")]
    executable: String,
    pub description: Option<String>,
}

impl<'de> Deserialize<'de> for Cmd {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut v: Value = Deserialize::deserialize(deserializer)?;
        if v.is_string() || v.is_sequence() {
            let mut m: Mapping = Mapping::new();
            m.insert(Value::String("run".to_string()), v.clone());
            v = Value::Mapping(m);
        }

        if v.is_mapping() {
            let c: Command = from_value(v).map_err(D::Error::custom)?;
            Ok(Cmd::from_command(c))
        } else {
            Err(D::Error::custom(
                "invalid type: commands must be a string, sequence, or map",
            ))
        }
    }
}

trait VecFromStr {
    fn from_str(s: &str) -> Self;
}

impl VecFromStr for Vec<String> {
    fn from_str(s: &str) -> Self {
        vec![s.to_string()]
    }
}

fn string_or_seq<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + VecFromStr,
    D: Deserializer<'de>,
{
    struct StringOrSeq<T>(PhantomData<fn() -> T>);

    impl<'de, T> Visitor<'de> for StringOrSeq<T>
    where
        T: Deserialize<'de> + VecFromStr,
    {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or sequence")
        }

        fn visit_str<E>(self, value: &str) -> Result<T, E>
        where
            E: de::Error,
        {
            Ok(VecFromStr::from_str(value))
        }

        fn visit_seq<S>(self, seq: S) -> Result<T, S::Error>
        where
            S: SeqAccess<'de>,
        {
            Deserialize::deserialize(de::value::SeqAccessDeserializer::new(seq))
        }
    }

    deserializer.deserialize_any(StringOrSeq(PhantomData))
}
