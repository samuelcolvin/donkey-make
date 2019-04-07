//macro_rules! str_err {
//    ($msg:expr) => (
//        Err($msg)
//    );
//    ($fmt:expr, $($arg:expr),+) => (
//        Err(format!($fmt, $($arg),+))
//    );
//}

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
