//! # The logger module
//!
//! This module exports macros to display messages to STDOUT or STDERR, depending on the set logging level.
//!
//! The logging level is stored inside [`static@LOGGER_LEVEL`], which can be atomically read and updated.

use atomic::Atomic;
use lazy_static::lazy_static;

lazy_static! {
    /// The minimum logging level of messages to display.
    /// All messages with a lower logging level won't be displayed.
    pub static ref LOGGER_LEVEL: Atomic<LoggerLevel> = Atomic::<LoggerLevel>::new(LoggerLevel::Error);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LoggerLevel {
    Failure,
    Error,
    Info,
    Debug,
}

/// Log a message if the logging level is high enough
#[macro_export]
macro_rules! log {
    ($logger_level: ident, $is_err: expr, $msg_prefix: expr, $msg: expr$(, $args: expr)*) => {{
        if $crate::logger::LOGGER_LEVEL.load(atomic::Ordering::SeqCst) >= $crate::logger::LoggerLevel::$logger_level {
            if $is_err {
                eprintln!(concat!($msg_prefix, $msg)$(, $args)*);
            } else {
                println!(concat!($msg_prefix, $msg)$(, $args)*);
            }
        }
    }}
}

/// Display a debug message (if logging level is high enough)
#[macro_export]
macro_rules! debug { ($msg: expr$(, $args: expr)*) => { $crate::log!(Debug, false, "[DEBUG] ", $msg$(, $args)*); } }

/// Display an information message (if logging level is high enough)
#[macro_export]
macro_rules! info { ($msg: expr$(, $args: expr)*) => { $crate::log!(Info, false, "[INFO] ", $msg$(, $args)*); } }

/// Display an error message (if logging level is high enough)
#[macro_export]
macro_rules! err { ($msg: expr$(, $args: expr)*) => { $crate::log!(Error, true, "[ERROR] ", $msg$(, $args)*); } }

/// Display a failure message and exit
#[macro_export]
macro_rules! fail { (exit $code: expr, $msg: expr$(, $args: expr)*) => {{
    $crate::log!(Failure, true, "[FAIL] ", $msg$(, $args)*);
    std::process::exit($code); }}
}
