macro_rules! exit {
    ($msg:expr) => (
        eprintln!("{}", Red.paint($msg));
        process::exit(1);
    );
    ($fmt:expr, $($arg:expr),+) => (
        let msg = format!($fmt, $($arg),+);
        eprintln!("{}", Red.paint(msg));
        process::exit(1);
    );
}

macro_rules! printlnc {
    ($colour:expr, $fmt:expr, $($arg:expr),+) => (
        let msg = format!($fmt, $($arg),+);
        println!("{}", Style::new().fg($colour).paint(msg));
    );
}
