//! Centralised error and exit‑handling utilities for the **TARDIS** CLI.
//!
//! This module provides a single [`Error`] enum that groups together all
//! *user* and *system* failures plus two convenience macros for constructing
//! those errors ergonomically.  It also offers the [`Failure`] trait, allowing
//! any error value to map itself to an appropriate process exit.  All public
//! items live behind concise documentation so that generated docs.rs output
//! remains immediately useful without excessive inline comments.

/// POSIX sysexits-compatible exit codes.
const EX_USAGE: i32 = 64; // Command line usage error
const EX_IOERR: i32 = 74; // Input/output error
const EX_CONFIG: i32 = 78; // Configuration error

/// All possible failures surfaced by the CLI.
#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum Error {
    /// Problems attributable to the user (bad flags, invalid input, …).
    #[error(transparent)]
    UserInput(#[from] UserInputError),
    /// Issues the user cannot fix without changing the environment
    /// (config corruption, I/O failures, …).
    #[error(transparent)]
    System(#[from] SystemError),
}

/// Human‑error variants.
#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum UserInputError {
    #[error("Invalid date format: {0}")]
    InvalidDateFormat(String),
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("Invalid date: {0}")]
    InvalidDate(String),
    #[error("Ambiguous datetime: {0}")]
    AmbiguousDateTime(String),
    #[error("Unsupported timezone: {0}")]
    UnsupportedTimezone(String),
    #[error("Invalid 'now' argument: {0}")]
    InvalidNow(String),
    #[error("Missing required argument: {0}")]
    MissingArgument(String),
}

/// Failures that stem from the operating environment or runtime.
#[derive(thiserror::Error, Debug)]
pub enum SystemError {
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Crate‑wide `Result` alias that uses the consolidated [`Error`] type.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Print a diagnostic message to stderr and exit with the appropriate code.
    pub fn exit(self) -> ! {
        match self {
            Error::UserInput(err) => {
                eprintln!("{}", err);
                std::process::exit(EX_USAGE);
            }
            Error::System(err) => {
                eprintln!("System error: {}", err);

                match err {
                    SystemError::Config(_) => std::process::exit(EX_CONFIG),
                    SystemError::Io(_) => std::process::exit(EX_IOERR),
                }
            }
        }
    }
}

impl PartialEq for SystemError {
    fn eq(&self, other: &Self) -> bool {
        use SystemError::*;
        match (self, other) {
            (Config(a), Config(b)) => a == b,
            (Io(a), Io(b)) => a.kind() == b.kind(),
            _ => false,
        }
    }
}

impl Eq for SystemError {}

/// Create an [`Error::UserInput`] of the requested variant with minimal boilerplate.
#[macro_export]
macro_rules! user_input_error {
    ($err_type:ident, $msg:expr) => {
        $crate::errors::Error::UserInput($crate::errors::UserInputError::$err_type($msg.to_string()))
    };

    ($err_type:ident, $($arg:tt)*) => {
        $crate::errors::Error::UserInput($crate::errors::UserInputError::$err_type(format!($($arg)*)))
    };

    ($err_type:ident) => {
        $crate::errors::Error::UserInput($crate::errors::UserInputError::$err_type(String::new()))
    };
}

/// Create an [`Error::System`] of the requested variant with minimal boilerplate.
#[macro_export]
macro_rules! system_error {
    ($err_type:ident, $msg:expr) => {
        $crate::errors::Error::System($crate::errors::SystemError::$err_type($msg.to_string()))
    };
    ($err_type:ident, $($arg:tt)*) => {
        $crate::errors::Error::System($crate::errors::SystemError::$err_type(format!($($arg)*)))
    };
    ($err_type:ident) => {
        $crate::errors::Error::System($crate::errors::SystemError::$err_type(String::new()))
    };
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn user_input_macro_literal() {
        let err = user_input_error!(InvalidDateFormat, "foo");
        assert!(matches!(
            err,
            Error::UserInput(UserInputError::InvalidDateFormat(ref s)) if s == "foo"
        ));
    }

    #[test]
    fn user_input_macro_formatted() {
        let err = user_input_error!(MissingArgument, "missing {}", "--format");
        assert!(matches!(
            err,
            Error::UserInput(UserInputError::MissingArgument(ref s)) if s == "missing --format"
        ));
    }

    #[test]
    fn user_input_macro_empty() {
        let err = user_input_error!(InvalidNow);
        assert!(matches!(
            err,
            Error::UserInput(UserInputError::InvalidNow(ref s)) if s.is_empty()
        ));
    }

    #[test]
    fn system_error_macro_literal() {
        let err = system_error!(Config, "invalid field");
        assert!(matches!(
            err,
            Error::System(SystemError::Config(ref s)) if s == "invalid field"
        ));
    }

    #[test]
    fn system_error_macro_formatted() {
        let err = system_error!(Config, "failed to read {}", "/tmp/foo");
        assert!(matches!(
            err,
            Error::System(SystemError::Config(ref s)) if s == "failed to read /tmp/foo"
        ));
    }

    #[test]
    fn system_error_macro_empty() {
        let err = system_error!(Config);
        assert!(matches!(
            err,
            Error::System(SystemError::Config(ref s)) if s.is_empty()
        ));
    }

    #[test]
    fn unsupported_format_error() {
        let err = user_input_error!(UnsupportedFormat, "bad format");
        assert!(matches!(
            err,
            Error::UserInput(UserInputError::UnsupportedFormat(ref s)) if s == "bad format"
        ));
    }

    #[test]
    fn conversion_from_io_error() {
        let err: Error = std::io::Error::from(std::io::ErrorKind::PermissionDenied).into();
        assert!(matches!(err, Error::System(SystemError::Io(_))));
    }

    #[test]
    fn system_error_partial_eq_config() {
        let a = SystemError::Config("x".into());
        let b = SystemError::Config("x".into());
        let c = SystemError::Config("y".into());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn system_error_partial_eq_io() {
        let a = SystemError::Io(std::io::Error::from(std::io::ErrorKind::NotFound));
        let b = SystemError::Io(std::io::Error::from(std::io::ErrorKind::NotFound));
        let c = SystemError::Io(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn system_error_partial_eq_different_variants() {
        let a = SystemError::Config("x".into());
        let b = SystemError::Io(std::io::Error::from(std::io::ErrorKind::NotFound));
        assert_ne!(a, b);
    }

    #[test]
    fn error_display_user_input() {
        let err = user_input_error!(InvalidDateFormat, "bad date");
        assert_eq!(format!("{err}"), "Invalid date format: bad date");
    }

    #[test]
    fn error_display_system() {
        let err = system_error!(Config, "broken");
        assert_eq!(format!("{err}"), "Configuration error: broken");
    }

    #[test]
    fn new_error_variants_display() {
        let err = user_input_error!(InvalidDate, "bad");
        assert_eq!(format!("{err}"), "Invalid date: bad");

        let err = user_input_error!(AmbiguousDateTime, "ambiguous");
        assert_eq!(format!("{err}"), "Ambiguous datetime: ambiguous");
    }
}
