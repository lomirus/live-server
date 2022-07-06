#![no_implicit_prelude]

extern crate colored;

macro_rules! info {
    ($($arg:tt)*) => ({
        let content = format!($($arg)*);
        println!("{}", content.bright_black());
    })
}

macro_rules! warning {
    ($($arg:tt)*) => ({
        let content = format!("[WARNING] {}", format!($($arg)*));
        println!("{}", content.yellow());
    })
}

macro_rules! error {
    ($($arg:tt)*) => ({
        use colored::Colorize;
        let content = format!("[ERROR] {}", format!($($arg)*));
        eprintln!("{}", content.red());
    })
}

macro_rules! panic {
    ($($arg:tt)*) => ({
        let content = format!("[PANIC] {}", format!($($arg)*));
        std::panic!("{}", content.red());
    })
}

pub(crate) use error;
pub(crate) use info;
pub(crate) use panic;
pub(crate) use warning;
