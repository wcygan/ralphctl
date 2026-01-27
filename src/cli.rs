//! CLI utility detection for ralphctl.
//!
//! Provides functions for detecting external CLI dependencies.

#![allow(dead_code)] // Utilities for init command

use std::process::Command;

/// Check if the `claude` CLI is available in PATH.
///
/// Uses the `which` command to locate the executable.
pub fn claude_exists() -> bool {
    Command::new("which")
        .arg("claude")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_exists_returns_bool() {
        // We can't assert the specific value since it depends on the environment,
        // but we can verify the function runs without panicking
        let _ = claude_exists();
    }

    #[test]
    fn test_which_nonexistent_command() {
        // Test that which returns false for a command that definitely doesn't exist
        let result = Command::new("which")
            .arg("definitely_not_a_real_command_abc123xyz")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);
        assert!(!result);
    }
}
