macro_rules! exit {
    ($msg:expr) => (
        eprintln!($msg);
        process::exit(1);
    );
    ($fmt:expr, $($arg:expr),+) => (
        eprintln!($fmt, $($arg),+);
        process::exit(1);
    );
}
