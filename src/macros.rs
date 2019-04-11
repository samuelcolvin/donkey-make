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
        println!("{}", ansi_term::Style::new().fg($colour).paint(msg));
    );
}
