use std::fs;
use std::io::Error;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread::{spawn, JoinHandle};
use std::time::{Duration, Instant};

use ansi_term::Colour::{Green, Yellow};
use linked_hash_map::LinkedHashMap as Map;
use nix::sys::signal::{kill, Signal as NixSignal};
use nix::unistd::Pid;
use notify::{raw_watcher, RawEvent, RecursiveMode, Watcher};

use crate::commands::{Cmd, Watch};
use crate::utils::{full_path, CliArgs};

pub struct Run {
    pub cmd_name: String,
    pub args: Vec<String>,
    pub env: Map<String, String>,
    pub working_dir: PathBuf,
    pub tmp_path: PathBuf,
    pub file_path: PathBuf,
    pub watch_path: Option<PathBuf>,
    pub print_summary: bool,
}

pub fn main(run: &Run, cmd: &Cmd, cli: &CliArgs) -> Result<i32, String> {
    let exit_code = match &cmd.watch {
        Some(Watch { debounce: d, .. }) => run_command_watch(&run, &cmd, *d),
        None => run_command_once(&run, &cmd),
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

fn run_command_once(run: &Run, cmd: &Cmd) -> Result<i32, String> {
    if run.print_summary {
        eprintlnc!(
            Green,
            "Running command \"{}\" from {}...",
            run.cmd_name,
            run.file_path.display()
        );
    }
    let sig = register_signals().map_err(error_str)?;
    let (exit_code, dur_str) = run_command(run, cmd)?;
    if let Some(c) = exit_code {
        Ok(c)
    } else {
        eprintlnc!(
            Yellow,
            "Command \"{}\" killed with signal {} after {} ✋",
            run.cmd_name,
            signal_name(&sig).unwrap_or("UNKNOWN"),
            dur_str
        );
        Ok(99)
    }
}

const WAIT_MS: u64 = 20;

fn run_command_watch(run: &Run, cmd: &Cmd, debounce: f32) -> Result<i32, String> {
    let watch_path = match &run.watch_path {
        Some(p) => p,
        _ => panic!("watch_path not set"),
    };
    eprintlnc!(
        Green,
        "Running command \"{}\" from {}, repeating on file changes in \"{}\"...",
        run.cmd_name,
        run.file_path.display(),
        full_path(watch_path)
    );
    // minimum time for which events will be grouped together
    let debounce_min = Duration::from_millis((debounce * 1000.0) as u64);
    // maximum time for which events will be grouped, if this time is reached the command will be restarted regardless
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
    let start = Instant::now();
    loop {
        if signal_name(&sig).is_some() {
            watch_stopped(&sig, &run.cmd_name, start.elapsed());
            return Ok(0);
        }
        let running_process = start_command(run, cmd)?;
        loop {
            if let Ok(evt) = rx.recv_timeout(recv_timeout) {
                events.push(evt);
                last_event = Some(Instant::now());
                if first_event.is_none() {
                    first_event = last_event;
                }
            }
            if signal_name(&sig).is_some() {
                watch_stopped(&sig, &run.cmd_name, start.elapsed());
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
        eprintlnc!(Green, "Restarting \"{}\"...", run.cmd_name);
        if !running_process.finished.load(Ordering::Relaxed) {
            let pid = Pid::from_raw(running_process.process_id);
            dbg!(pid);
            kill(pid, NixSignal::SIGTERM).unwrap();
        }
        running_process
            .handle
            .join()
            .expect("Unable to join await_command thread")?;

        println!("event: {:?}", events);
        first_event = None;
        last_event = None;
        events.clear();
    }
}

fn watch_stopped(sig: &Signal, cmd_name: &str, duration: Duration) {
    eprintlnc!(
        Green,
        "Running \"{}\" stopped with signal {} after {}",
        cmd_name,
        signal_name(sig).unwrap_or("UNKNOWN"),
        format_duration(duration)
    );
}

fn run_command(run: &Run, cmd: &Cmd) -> Result<(Option<i32>, String), String> {
    let rp = start_command(run, cmd)?;
    rp.handle.join().expect("Unable to join await_command thread")
}

struct RunningProcess {
    pub process_id: i32,
    pub finished: Arc<AtomicBool>,
    pub handle: JoinHandle<Result<(Option<i32>, String), String>>,
}

fn start_command(run: &Run, cmd: &Cmd) -> Result<RunningProcess, String> {
    let mut c = Command::new(cmd.executable());
    c.args(&run.args).envs(&run.env).current_dir(&run.working_dir);

    let cmd_name = run.cmd_name.clone();
    let print_summary = run.print_summary;
    let start = Instant::now();
    let mut p = c.spawn().map_err(error_str)?;
    let process_id = p.id() as i32;
    let finished = Arc::new(AtomicBool::new(false));
    let finished_clone = Arc::clone(&finished);
    let handle = spawn(move || await_command(&mut p, cmd_name, print_summary, start, finished_clone));
    let rp = RunningProcess {
        process_id,
        finished,
        handle,
    };
    Ok(rp)
}

fn await_command(
    p: &mut Child,
    cmd_name: String,
    print_summary: bool,
    start: Instant,
    finished: Arc<AtomicBool>,
) -> Result<(Option<i32>, String), String> {
    let status = p.wait().map_err(error_str)?;
    let duration = start.elapsed();
    let dur_str = format_duration(duration);
    finished.store(true, Ordering::Relaxed);
    if let Some(c) = status.code() {
        if c == 0 {
            if print_summary {
                eprintlnc!(Green, "Command \"{}\" successful in {} 👍", cmd_name, dur_str);
            }
        } else {
            eprintlnc!(
                Yellow,
                "Command \"{}\" failed in {}, exit code {} 👎",
                cmd_name,
                dur_str,
                c
            );
        }
    }
    Ok((status.code(), dur_str))
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
