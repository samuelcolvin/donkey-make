macro_rules! err {
    ($msg:expr) => (
        Err(format!("{}", $msg))
    );
    ($fmt:expr, $($arg:expr),+) => (
        Err(format!($fmt, $($arg),+))
    );
}

macro_rules! printlnc {
    ($colour:expr, $fmt:expr, $($arg:expr),+) => (
        let msg = format!($fmt, $($arg),+);
        if atty::is(atty::Stream::Stdout) {
            println!("{}", $colour.paint(msg));
        } else {
            println!("{}", msg);
        }
    );
}

macro_rules! eprintlnc {
    ($colour:expr, $fmt:expr, $($arg:expr),+) => (
        let msg = format!($fmt, $($arg),+);
        if atty::is(atty::Stream::Stderr) {
            eprintln!("{}", $colour.paint(msg));
        } else {
            eprintln!("{}", msg);
        }
    );
}

macro_rules! paint {
    ($colour:expr, $msg:expr) => {
        if atty::is(atty::Stream::Stdout) {
            $colour.paint($msg)
        } else {
            ansi_term::Style::default().paint($msg)
        }
    };
}
