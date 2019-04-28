use std::env;
use std::fmt;
use std::fs::File;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use linked_hash_map::LinkedHashMap as Map;
use serde::de::{self, Deserialize, Deserializer, Error, SeqAccess, Visitor};
use serde_yaml::{from_reader, from_value, Mapping, Value};

use crate::consts::{BASH, BASH_SMART, DONKEY_FILE_ENV};

#[derive(Debug, Deserialize)]
pub struct FileConfig {
    #[serde(rename = ".env")]
    #[serde(default)]
    pub env: Map<String, String>,

    #[serde(flatten)]
    pub commands: Map<String, Cmd>,
}

impl FileConfig {
    pub fn keys(&self) -> Vec<String> {
        self.commands.keys().cloned().collect()
    }
}

fn dft_interval() -> f32 {
    1.0
}

#[derive(Debug, Deserialize)]
#[serde(tag = "mode")]
pub enum Repeat {
    #[serde(rename = "periodic")]
    Periodic {
        #[serde(default = "dft_interval")]
        interval: f32,
    },
    #[serde(rename = "watch")]
    Watch {
        #[serde(default = "dft_interval")]
        interval: f32,
        dir: String,
    },
}

#[derive(Debug)]
pub struct Cmd {
    pub run: Vec<String>,
    pub args: Vec<String>,
    pub env: Map<String, String>,
    pub working_dir: Option<String>,
    pub repeat: Option<Repeat>,
    executable: String,
    description: Option<String>,
}

impl Cmd {
    pub fn smart(&self) -> bool {
        match self.executable.as_ref() {
            BASH_SMART => true,
            _ => false,
        }
    }

    pub fn executable(&self) -> String {
        if self.smart() {
            BASH.to_string()
        } else {
            self.executable.clone()
        }
    }

    fn first_line(&self) -> String {
        match self.run.first() {
            Some(f) => {
                let mut first_line = f.clone();
                let mut more = match self.run.len() {
                    c if c > 1 => true,
                    _ => false,
                };
                if let Some(nl) = first_line.find('\n') {
                    more = true;
                    first_line = first_line[..nl].to_string();
                }
                if more {
                    format!("{}…", first_line.trim_end())
                } else if first_line.chars().count() > 40 {
                    format!("{}…", first_line[..39].to_string().trim_end())
                } else {
                    first_line
                }
            }
            None => "".to_string(),
        }
    }

    pub fn description(&self) -> String {
        match &self.description {
            Some(d) => d.clone(),
            None => self.first_line(),
        }
    }

    pub fn summary(&self) -> String {
        let ex_str = if self.smart() {
            "".to_string()
        } else {
            format!("{}, ", self.executable())
        };
        let lines = match self.run.len() {
            1 => "1 line".to_string(),
            c => format!("{} lines", c),
        };
        format!("({}{})", ex_str, lines)
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

pub fn find_file(file_path_opt: &Option<String>) -> Result<PathBuf, String> {
    if let Some(file_path) = file_path_opt {
        return Ok(PathBuf::from(file_path.clone()));
    }
    if let Ok(p) = env::var(DONKEY_FILE_ENV) {
        return Ok(PathBuf::from(p));
    }
    for path in PATH_OPTIONS.iter() {
        let path_option = Path::new(path);
        if path_option.exists() {
            return Ok(path_option.to_path_buf());
        }
    }
    err!(
        "No commands config file provided, and no default found, tried:\n  \
         donk.ya?ml, donkey.ya?ml and donkey-make.ya?ml"
    )
}

pub fn load_file(path: &PathBuf) -> Result<FileConfig, String> {
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

impl<'de> Deserialize<'de> for Cmd {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        fn dft_exe() -> String {
            BASH_SMART.to_string()
        }

        #[derive(Debug, Deserialize)]
        struct Command {
            #[serde(deserialize_with = "seq_or_string")]
            run: Vec<String>,
            #[serde(default)]
            args: Vec<String>,
            #[serde(default)]
            env: Map<String, String>,
            working_dir: Option<String>,
            repeat: Option<Repeat>,
            #[serde(rename = "ex")]
            #[serde(default = "dft_exe")]
            executable: String,
            description: Option<String>,
        }

        let mut v: Value = Deserialize::deserialize(deserializer)?;
        if v.is_string() || v.is_sequence() {
            let mut m: Mapping = Mapping::with_capacity(1);
            m.insert(Value::String("run".to_string()), v.clone());
            v = Value::Mapping(m);
        }

        if v.is_mapping() {
            let c: Command = from_value(v).map_err(D::Error::custom)?;
            Ok(Cmd {
                run: c.run,
                args: c.args,
                env: c.env,
                working_dir: c.working_dir,
                repeat: c.repeat,
                executable: c.executable,
                description: c.description,
            })
        } else {
            Err(D::Error::custom(
                "invalid type: commands must be a string, sequence, or map",
            ))
        }
    }
}

trait SeqFromStr {
    fn from_str(s: &str) -> Self;
}

impl SeqFromStr for Vec<String> {
    fn from_str(s: &str) -> Self {
        vec![s.to_string()]
    }
}

fn seq_or_string<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + SeqFromStr,
    D: Deserializer<'de>,
{
    struct StringOrSeq<T>(PhantomData<fn() -> T>);

    impl<'de, T> Visitor<'de> for StringOrSeq<T>
    where
        T: Deserialize<'de> + SeqFromStr,
    {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or sequence")
        }

        fn visit_str<E>(self, value: &str) -> Result<T, E>
        where
            E: de::Error,
        {
            Ok(SeqFromStr::from_str(value))
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
