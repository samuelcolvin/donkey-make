use std::fs;
use std::io::Error;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};

use ansi_term::Colour::{Green, Yellow};
use linked_hash_map::LinkedHashMap as Map;
use notify::{raw_watcher, RawEvent, RecursiveMode, Watcher};

use crate::commands::{Cmd, Repeat};
use crate::consts::CliArgs;

pub struct Run {
    pub cmd_name: String,
    pub args: Vec<String>,
    pub env: Map<String, String>,
    pub working_dir: PathBuf,
    pub tmp_path: PathBuf,
    pub file_path: PathBuf,
    pub print_summary: bool,
}

pub fn main(run: &Run, cmd: &Cmd, cli: &CliArgs) -> Result<i32, String> {
    let exit_code = match &cmd.repeat {
        Some(Repeat::Periodic { interval: i }) => run_command_periodic(&run, &cmd, *i).map_err(error_str),
        Some(Repeat::Watch { debounce: d, path }) => run_command_watch(&run, &cmd, *d, path),
        None => run_command_once(&run, &cmd).map_err(error_str),
    };
    delete(&run.tmp_path, cli.keep_tmp)?;
    match exit_code {
        Ok(t) => Ok(t),
        Err(e) => err!(
            "failed to execute command \"{} {}\": {}",
            cmd.executable(),
            run.args.join(" "),
            e
        ),
    }
}

fn run_command_once(run: &Run, cmd: &Cmd) -> Result<i32, Error> {
    if run.print_summary {
        eprintlnc!(
            Green,
            "Running command \"{}\" from {}...",
            run.cmd_name,
            run.file_path.display()
        );
    }
    let sig = register_signals()?;
    run_command(run, cmd, &sig)
}

const WAIT_MS: u64 = 20;

fn run_command_periodic(run: &Run, cmd: &Cmd, interval: f32) -> Result<i32, Error> {
    eprintlnc!(
        Green,
        "Running command \"{}\" from {}, repeating at {:0.2}s intervals...",
        run.cmd_name,
        run.file_path.display(),
        interval
    );
    let sleep_steps = (interval * 1000.0 / (WAIT_MS as f32)) as u64;
    let sleep_time = Duration::from_millis(WAIT_MS);
    let sig = register_signals()?;

    loop {
        let status_code = run_command(run, cmd, &sig)?;
        if status_code != 0 {
            return Ok(status_code);
        }
        for _ in 0..sleep_steps {
            sleep(sleep_time);
            if signal_name(&sig).is_some() {
                return Ok(0);
            }
        }
    }
}

fn run_command_watch(run: &Run, cmd: &Cmd, debounce: f32, watch_path: &str) -> Result<i32, String> {
    eprintlnc!(
        Green,
        "Running command \"{}\" from {}, repeating on file changes in \"{}\"...",
        run.cmd_name,
        run.file_path.display(),
        watch_path
    );
    // minimum time for which events will be grouped together
    let debounce_min = Duration::from_millis((debounce * 1000.0) as u64);
    // maximum time for which events will be grouped, if this time is reached the command will be run regardless
    // of whether an event happened recently
    let debounce_max = debounce_min * 4;
    let recv_timeout = Duration::from_millis(WAIT_MS);

    let (tx, rx) = channel();
    let mut watcher = raw_watcher(tx).map_err(error_str)?;
    watcher.watch(watch_path, RecursiveMode::Recursive).map_err(error_str)?;
    let sig = register_signals().map_err(error_str)?;

    let mut first_event: Option<Instant> = None;
    let mut last_event: Option<Instant> = None;
    let mut events: Vec<RawEvent> = Vec::new();
    loop {
        loop {
            if let Ok(evt) = rx.recv_timeout(recv_timeout) {
                events.push(evt);
                last_event = Some(Instant::now());
                if first_event.is_none() {
                    first_event = last_event;
                }
            }
            if signal_name(&sig).is_some() {
                return Ok(0);
            }
            if let Some(i) = last_event {
                if i.elapsed() > debounce_min {
                    break;
                }
            }
            if let Some(i) = first_event {
                if i.elapsed() > debounce_max {
                    break;
                }
            }
        }
        println!("event: {:?}", events);
        first_event = None;
        last_event = None;
        events.clear();
        let status_code = run_command(run, cmd, &sig).map_err(error_str)?;
        if status_code != 0 {
            return Ok(status_code);
        }
        if signal_name(&sig).is_some() {
            return Ok(0);
        }
    }
}

fn run_command(run: &Run, cmd: &Cmd, sig: &Signal) -> Result<i32, Error> {
    let mut c = Command::new(cmd.executable());
    c.args(&run.args).envs(&run.env).current_dir(&run.working_dir);

    let start = Instant::now();
    let status = c.status()?;
    let duration = start.elapsed();
    let dur_str = format_duration(duration);
    if let Some(c) = status.code() {
        if c == 0 {
            if run.print_summary {
                eprintlnc!(Green, "Command \"{}\" successful in {} 👍", run.cmd_name, dur_str);
            }
        } else {
            eprintlnc!(
                Yellow,
                "Command \"{}\" failed in {}, exit code {} 👎",
                run.cmd_name,
                dur_str,
                c
            );
        }
        Ok(c)
    } else {
        eprintlnc!(
            Yellow,
            "Command \"{}\" kill with signal {} after {} ✋",
            run.cmd_name,
            signal_name(sig).unwrap_or("UNKNOWN"),
            dur_str
        );
        Ok(99)
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

fn signal_name(sig: &Signal) -> Option<&'static str> {
    if sig.int.load(Ordering::Relaxed) {
        Some("SIGINT")
    } else if sig.term.load(Ordering::Relaxed) {
        Some("SIGTERM")
    } else {
        None
    }
}

fn error_str<T>(e: T) -> String
where
    T: std::fmt::Display,
{
    format!("{}", e)
}

fn format_duration(duration: Duration) -> String {
    match duration {
        d if d < Duration::from_millis(10) => format!("{:0.3}ms", d.subsec_micros() as f32 / 1000.0),
        d if d < Duration::from_secs(1) => format!("{}ms", d.subsec_millis()),
        d if d < Duration::from_secs(100) => {
            format!("{:0.3}s", d.as_secs() as f64 + f64::from(d.subsec_millis()) / 1000.0)
        }
        d => format!("{}s", d.as_secs()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    #[test]
    fn format_duration_5ms() {
        let tic = SystemTime::now();
        let toc = tic + Duration::from_millis(5);
        assert_eq!(format_duration(toc.duration_since(tic).unwrap()), "5.000ms");
    }

    #[test]
    fn format_duration_15ms() {
        let tic = SystemTime::now();
        let toc = tic + Duration::from_millis(15);
        assert_eq!(format_duration(toc.duration_since(tic).unwrap()), "15ms");
    }

    #[test]
    fn format_duration_2s() {
        let tic = SystemTime::now();
        let toc = tic + Duration::from_secs(2);
        assert_eq!(format_duration(toc.duration_since(tic).unwrap()), "2.000s");
    }

    #[test]
    fn format_duration_200s() {
        let tic = SystemTime::now();
        let toc = tic + Duration::from_secs(200);
        assert_eq!(format_duration(toc.duration_since(tic).unwrap()), "200s");
    }
}
