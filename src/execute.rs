use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::Error;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use ansi_term::Colour::{Cyan, Green, Yellow};
use linked_hash_map::LinkedHashMap as Map;

use crate::commands::{Cmd, FileConfig};
use crate::consts::{CliArgs, BAR, DONKEY_COMMAND_ENV, DONKEY_DEPTH_ENV, DONKEY_FILE_ENV, DONKEY_KEEP_ENV, PATH_STR};

pub fn main(
    cmd_name: &str,
    config: &FileConfig,
    cmd: &Cmd,
    cli: &CliArgs,
    file_path: &PathBuf,
) -> Result<Option<i32>, String> {
    let mut path_str: String = PATH_STR.to_string();
    let mut run_depth: i32 = 0;
    if let Ok(v) = env::var(DONKEY_DEPTH_ENV) {
        path_str = format!("{}.{}", PATH_STR, v);
        run_depth = v.parse::<i32>().unwrap_or(1);
    }
    let smart_prefix = match env::var(DONKEY_COMMAND_ENV) {
        Ok(c) => format!("{} > {}", c, cmd_name),
        _ => cmd_name.to_string(),
    };
    let mut args: Vec<String> = vec![path_str.clone()];
    args.extend(cmd.args.iter().cloned());
    args.extend(cli.args.iter().cloned());

    let mut env: StrMap = Map::new();
    merge_maps(&mut env, &config.env);
    merge_maps(&mut env, &cmd.env);
    env.insert(DONKEY_DEPTH_ENV.to_string(), (run_depth + 1).to_string());
    env.insert(DONKEY_FILE_ENV.to_string(), full_path(file_path));
    env.insert(DONKEY_COMMAND_ENV.to_string(), smart_prefix.clone());
    env.insert(
        DONKEY_KEEP_ENV.to_string(),
        String::from(if cli.keep_tmp { "1" } else { "0" }),
    );

    let working_dir = get_working_dir(&cmd, file_path)?;
    let path = working_dir.join(&path_str);
    write(cmd_name, &path, cmd, &args, &env, config, smart_prefix)?;

    let print_summary: bool = run_depth == 0;
    if print_summary {
        eprintlnc!(
            Green,
            r#"Running command "{}" from {}..."#,
            cmd_name,
            file_path.display()
        );
    }

    let exit_code = run_command(cmd_name, cmd, &args, &env, working_dir, print_summary);
    delete(&path, cli.keep_tmp)?;
    match exit_code {
        Ok(t) => Ok(t),
        Err(e) => err!(
            "failed to execute command \"{} {}\": {}",
            cmd.executable(),
            args.join(" "),
            e
        ),
    }
}

fn write(
    cmd_name: &str,
    path: &PathBuf,
    cmd: &Cmd,
    args: &[String],
    env: &StrMap,
    config: &FileConfig,
    smart_prefix: String,
) -> Result<(), String> {
    if path.exists() {
        return err!(
            "Error writing temporary file:\n  {} already exists, donkey-make may be running already",
            path.display()
        );
    }

    let prefix: Vec<String> = vec![
        String::from(BAR),
        format!(
            "This is a temporary file generated by donkey-make to execute the command: \"{}\"",
            cmd_name
        ),
        format!("Command to be executed: \"{} {}\"", cmd.executable(), args.join(" ")),
        String::from("Environment variables set:"),
        format!("{:?}", env),
        String::from("This file should only exist very temporarily while it's be executed."),
        String::from(BAR),
    ];
    let comment = if cmd.executable().starts_with("node") {
        "//"
    } else {
        "#"
    };
    let sep = format!("\n{} ", comment);

    let script: String = if cmd.smart() {
        let donk_exe = match env::current_exe() {
            Ok(ex) => full_path(&ex),
            Err(e) => return err!("finding current executable for smart script failed: {}", e),
        };
        let mut cmd_tree: HashSet<String> = HashSet::new();
        cmd_tree.insert((*cmd_name).to_string());
        build_smart_script(&cmd, smart_prefix, &donk_exe, config, &mut cmd_tree)?
    } else {
        cmd.run.join("\n")
    };

    let content = format!("{} {}\n{}", comment, prefix.join(&sep), script);

    match create_file(path, &content) {
        Ok(_) => Ok(()),
        Err(e) => err!("Error writing temporary file {}:\n  {}", path.display(), e),
    }
}

fn run_command(
    cmd_name: &str,
    cmd: &Cmd,
    args: &[String],
    env: &StrMap,
    working_dir: PathBuf,
    print_summary: bool,
) -> Result<Option<i32>, Error> {
    let mut c = Command::new(&cmd.executable());
    c.args(args).envs(env).current_dir(working_dir);
    let sig = register_signals()?;

    let tic = SystemTime::now();
    let status = c.status()?;
    let toc = SystemTime::now();
    let dur_str = format_duration(tic, toc);
    if status.success() {
        if print_summary {
            eprintlnc!(Green, "Command \"{}\" successful in {} 👍", cmd_name, dur_str);
        }
        Ok(None)
    } else {
        if print_summary {
            if let Some(c) = status.code() {
                eprintlnc!(
                    Yellow,
                    "Command \"{}\" failed in {}, exit code {} 👎",
                    cmd_name,
                    dur_str,
                    c
                );
            } else {
                eprintlnc!(
                    Yellow,
                    "Command \"{}\" kill with signal {} after {} 👎",
                    cmd_name,
                    signal_name(sig),
                    dur_str
                );
            }
        }
        match status.code() {
            Some(c) => Ok(Some(c)),
            None => Ok(Some(99)),
        }
    }
}

fn delete(path: &PathBuf, keep: bool) -> Result<(), String> {
    if !keep {
        match fs::remove_file(path) {
            Ok(t) => t,
            Err(e) => {
                return err!("Error deleting temporary file {}, {}", path.display(), e);
            }
        };
    }
    Ok(())
}

fn create_file(path: &Path, content: &str) -> std::io::Result<()> {
    let mut f = fs::File::create(path)?;
    f.write_all(content.as_bytes())?;
    Ok(())
}

struct Signal {
    int: Arc<AtomicBool>,
    term: Arc<AtomicBool>,
}

fn register_signals() -> Result<Signal, Error> {
    let sig = Signal {
        int: Arc::new(AtomicBool::new(false)),
        term: Arc::new(AtomicBool::new(false)),
    };
    // TODO this doesn't forward the signal to the child, but generally the terminal does that for us
    signal_hook::flag::register(signal_hook::SIGINT, Arc::clone(&sig.int))?;
    signal_hook::flag::register(signal_hook::SIGTERM, Arc::clone(&sig.term))?;
    Ok(sig)
}

fn signal_name(sig: Signal) -> &'static str {
    if sig.int.load(Ordering::Relaxed) {
        "SIGINT"
    } else if sig.term.load(Ordering::Relaxed) {
        "SIGTERM"
    } else {
        "UNKNOWN"
    }
}

const NO_ECHO_PREFIX: char = '_';
const DONK_PREFIX: char = '+';
const INLINE_PREFIX: char = '<';
const PREFIXES: [char; 3] = [NO_ECHO_PREFIX, DONK_PREFIX, INLINE_PREFIX];

fn build_smart_script(
    cmd: &Cmd,
    smart_prefix: String,
    donk_exe: &str,
    config: &FileConfig,
    cmd_tree: &mut HashSet<String>,
) -> Result<String, String> {
    let all = cmd.run.join("\n");
    let lines: Vec<&str> = all.split('\n').collect();
    let len = lines.len();

    let mut script: Vec<String> = vec!["set -e".to_string()];
    for line in lines {
        if !PREFIXES.iter().any(|&prefix| line.starts_with(prefix)) {
            let coloured = epaint!(Cyan, format!("{} > {}", smart_prefix, line));
            script.push(format!(">&2 echo '{}'", coloured));
        }

        let mut ex_line = if line.starts_with(NO_ECHO_PREFIX) {
            line[1..].to_string()
        } else {
            line.to_string()
        };

        if ex_line.starts_with(INLINE_PREFIX) {
            let sub_cmd_name = &ex_line[1..].trim().to_string();
            if cmd_tree.contains(sub_cmd_name) {
                return err!(
                    "Command \"{}\" reused in an inline sub-command, this would cause infinite recursion",
                    sub_cmd_name
                );
            }
            cmd_tree.insert(sub_cmd_name.clone().to_string());
            let sub_cmd = get_sub_command(config, sub_cmd_name)?;
            let sub_cmd_prefix = format!("{} > {}", smart_prefix, sub_cmd_name);
            ex_line = build_smart_script(sub_cmd, sub_cmd_prefix, donk_exe, config, &mut *cmd_tree)?;
        } else {
            if len == 1 && !line.contains('$') {
                // must be the first line
                ex_line = format!("{} $@", line)
            }
            if ex_line.starts_with(DONK_PREFIX) {
                ex_line = format!("{} {}", donk_exe, &ex_line[1..]);
            }
        }
        script.push(ex_line);
    }
    Ok(script.join("\n"))
}

fn get_sub_command<'a>(config: &'a FileConfig, cmd_name: &str) -> Result<&'a Cmd, String> {
    match config.commands.get(cmd_name) {
        Some(c) => {
            if c.smart() {
                Ok(c)
            } else {
                err!(
                    "Sub-command \"{}\" not a bash-smart script, remove \"ex:\" or use '{}' not '{}'",
                    cmd_name,
                    DONK_PREFIX,
                    INLINE_PREFIX
                )
            }
        }
        None => err!(
            "Sub-command \"{}\" not found, commands available are:\n  {}",
            cmd_name,
            config.keys().join(", ")
        ),
    }
}

type StrMap = Map<String, String>;

fn merge_maps(base: &mut StrMap, update: &StrMap) {
    base.extend(update.iter().map(|(k, v)| (k.clone(), v.clone())));
}

fn format_duration(tic: SystemTime, toc: SystemTime) -> String {
    match toc.duration_since(tic).unwrap_or(Duration::from_secs(0)) {
        d if d < Duration::from_millis(10) => format!("{:0.3}ms", d.subsec_micros() as f32 / 1000.0),
        d if d < Duration::from_secs(1) => format!("{}ms", d.subsec_millis()),
        d if d < Duration::from_secs(100) => {
            format!("{:0.3}s", d.as_secs() as f64 + f64::from(d.subsec_millis()) / 1000.0)
        }
        d => format!("{}s", d.as_secs()),
    }
}

fn full_path(path: &PathBuf) -> String {
    match path.canonicalize() {
        Ok(p) => p.to_string_lossy().to_string(),
        _ => path.to_string_lossy().to_string(),
    }
}

fn get_working_dir(cmd: &Cmd, file_path: &PathBuf) -> Result<PathBuf, String> {
    match &cmd.working_dir {
        Some(wd) => {
            let mut path = PathBuf::from(&wd);
            if path.is_relative() {
                let file_dir = match file_path.parent() {
                    Some(p) => p,
                    _ => return err!("file path appears to have no parent directory"),
                };
                path = file_dir.join(&wd).to_path_buf();
            }
            if !path.is_dir() {
                err!("\"{}\" is not a directory", &wd)
            } else {
                Ok(match path.canonicalize() {
                    Ok(p) => p,
                    _ => path,
                })
            }
        }
        _ => match env::current_dir() {
            Ok(p) => Ok(p),
            Err(e) => err!("unable to resolve current working directory: {}", e),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linked_hash_map::LinkedHashMap as Map;
    use std::time::{Duration, SystemTime};

    #[test]
    fn format_duration_5ms() {
        let tic = SystemTime::now();
        let toc = tic + Duration::from_millis(5);
        assert_eq!(format_duration(tic, toc), "5.000ms");
    }

    #[test]
    fn format_duration_15ms() {
        let tic = SystemTime::now();
        let toc = tic + Duration::from_millis(15);
        assert_eq!(format_duration(tic, toc), "15ms");
    }

    #[test]
    fn format_duration_2s() {
        let tic = SystemTime::now();
        let toc = tic + Duration::from_secs(2);
        assert_eq!(format_duration(tic, toc), "2.000s");
    }

    #[test]
    fn format_duration_200s() {
        let tic = SystemTime::now();
        let toc = tic + Duration::from_secs(200);
        assert_eq!(format_duration(tic, toc), "200s");
    }

    #[test]
    fn format_duration_negative() {
        let tic = SystemTime::now();
        let toc = tic + Duration::from_millis(150);
        assert_eq!(format_duration(toc, tic), "0.000ms");
    }

    #[test]
    fn merge_add() {
        let mut base: Map<String, String> = Map::new();
        base.insert("a".to_string(), "b".to_string());
        let mut update: Map<String, String> = Map::new();
        update.insert("c".to_string(), "d".to_string());
        merge_maps(&mut base, &update);
        assert_eq!(format!("{:?}", base), r#"{"a": "b", "c": "d"}"#);
    }

    #[test]
    fn merge_update() {
        let mut base: Map<String, String> = Map::new();
        base.insert("a".to_string(), "b".to_string());
        let mut update: Map<String, String> = Map::new();
        update.insert("a".to_string(), "d".to_string());
        merge_maps(&mut base, &update);
        assert_eq!(format!("{:?}", base), r#"{"a": "d"}"#);
    }
}
