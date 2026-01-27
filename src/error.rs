//! Error handling utilities for ralphctl.
//!
//! Provides terse Unix-style error formatting and exit code constants.

#![allow(dead_code)] // Utilities for future command implementations

use std::process;

/// Exit codes following Unix conventions and CLI spec
pub mod exit {
    /// Successful completion
    pub const SUCCESS: i32 = 0;
    /// General error (missing files, invalid input, blocked)
    pub const ERROR: i32 = 1;
    /// Max iterations reached without completion
    pub const MAX_ITERATIONS: i32 = 2;
    /// Interrupted by signal (Ctrl+C)
    pub const INTERRUPTED: i32 = 130;
}

/// Print an error message to stderr in Unix style and exit.
///
/// Format: `error: <message>`
pub fn die(msg: &str) -> ! {
    eprintln!("error: {}", msg);
    process::exit(exit::ERROR);
}

/// Print an error message to stderr in Unix style and exit with a specific code.
pub fn die_with_code(msg: &str, code: i32) -> ! {
    eprintln!("error: {}", msg);
    process::exit(code);
}

/// Extension trait for adding terse context to Results.
pub trait ResultExt<T> {
    /// Add context to an error, converting it to anyhow::Error.
    fn context_terse(self, msg: &str) -> anyhow::Result<T>;
}

impl<T, E: std::error::Error + Send + Sync + 'static> ResultExt<T> for Result<T, E> {
    fn context_terse(self, msg: &str) -> anyhow::Result<T> {
        self.map_err(|_| anyhow::anyhow!("{}", msg))
    }
}

impl<T> ResultExt<T> for Option<T> {
    fn context_terse(self, msg: &str) -> anyhow::Result<T> {
        self.ok_or_else(|| anyhow::anyhow!("{}", msg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_ext_ok() {
        let result: Result<i32, std::io::Error> = Ok(42);
        assert_eq!(result.context_terse("should not appear").unwrap(), 42);
    }

    #[test]
    fn test_result_ext_err() {
        let result: Result<i32, std::io::Error> = Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "original",
        ));
        let err = result.context_terse("file not found").unwrap_err();
        assert_eq!(err.to_string(), "file not found");
    }

    #[test]
    fn test_option_ext_some() {
        let opt: Option<i32> = Some(42);
        assert_eq!(opt.context_terse("should not appear").unwrap(), 42);
    }

    #[test]
    fn test_option_ext_none() {
        let opt: Option<i32> = None;
        let err = opt.context_terse("value missing").unwrap_err();
        assert_eq!(err.to_string(), "value missing");
    }
}
