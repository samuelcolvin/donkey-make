macro_rules! exit {
    ($msg:expr) => (
        eprintln!("{}", ansi_term::Colour::Red.paint($msg));
        std::process::exit(1);
    );
    ($fmt:expr, $($arg:expr),+) => (
        let msg = format!($fmt, $($arg),+);
        eprintln!("{}", ansi_term::Colour::Red.paint(msg));
        std::process::exit(1);
    );
}

macro_rules! printlnc {
    ($colour:expr, $fmt:expr, $($arg:expr),+) => (
        let msg = format!($fmt, $($arg),+);
        println!("{}", ansi_term::Style::new().fg($colour).paint(msg));
    );
}
