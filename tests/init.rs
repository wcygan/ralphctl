//! Integration tests for the `ralphctl init` command.
//!
//! Note: Some tests require the `claude` CLI to be installed. Tests that depend
//! on claude being present are marked with `#[ignore]` and can be run with
//! `cargo test -- --ignored` when claude is available.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Get a command for ralphctl.
fn ralphctl() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("ralphctl"))
}

/// Create a temporary directory for testing.
fn temp_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

/// Check if claude CLI is available in the current environment.
fn claude_available() -> bool {
    std::process::Command::new("which")
        .arg("claude")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn init_fails_without_claude_cli() {
    let dir = temp_dir();

    // Set PATH to a minimal value that excludes claude
    // Include /usr/bin for 'which' to work, but not typical claude locations
    ralphctl()
        .current_dir(dir.path())
        .env("PATH", "/usr/bin")
        .arg("init")
        .assert()
        .failure()
        .stderr(predicate::str::contains("claude not found in PATH"));
}

#[test]
fn init_fails_when_files_exist_without_force() {
    let dir = temp_dir();

    // Create existing SPEC.md
    fs::write(dir.path().join("SPEC.md"), "# Existing Spec").unwrap();

    // init should fail - either because claude not found or files exist
    ralphctl()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .failure();
}

#[test]
fn init_help_shows_force_flag() {
    ralphctl()
        .arg("init")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--force"));
}

#[test]
fn init_help_describes_force() {
    ralphctl()
        .arg("init")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Overwrite existing files"));
}

// Tests that require claude to be installed
// Run with: cargo test -- --ignored
#[cfg(unix)]
mod requires_claude {
    use super::*;

    /// Skip test if claude is not available
    fn skip_if_no_claude() -> bool {
        if !claude_available() {
            eprintln!("Skipping test: claude CLI not available");
            true
        } else {
            false
        }
    }

    #[test]
    fn init_with_existing_files_shows_error() {
        if skip_if_no_claude() {
            return;
        }

        let dir = temp_dir();

        // Create existing SPEC.md
        fs::write(dir.path().join("SPEC.md"), "# Existing Spec").unwrap();

        // With claude available, should fail due to file existence
        ralphctl()
            .current_dir(dir.path())
            .arg("init")
            .assert()
            .failure()
            .stderr(predicate::str::contains("files already exist"));
    }

    #[test]
    fn init_error_lists_existing_files() {
        if skip_if_no_claude() {
            return;
        }

        let dir = temp_dir();

        // Create multiple existing files
        fs::write(dir.path().join("SPEC.md"), "# Existing").unwrap();
        fs::write(dir.path().join("PROMPT.md"), "# Existing").unwrap();

        // Error should mention which files exist
        ralphctl()
            .current_dir(dir.path())
            .arg("init")
            .assert()
            .failure()
            .stderr(predicate::str::contains("SPEC.md"))
            .stderr(predicate::str::contains("PROMPT.md"));
    }

    #[test]
    fn init_error_suggests_force_flag() {
        if skip_if_no_claude() {
            return;
        }

        let dir = temp_dir();

        fs::write(dir.path().join("SPEC.md"), "# Existing").unwrap();

        // Error should suggest using --force
        ralphctl()
            .current_dir(dir.path())
            .arg("init")
            .assert()
            .failure()
            .stderr(predicate::str::contains("--force"));
    }

    #[test]
    fn init_with_all_files_lists_all_in_error() {
        if skip_if_no_claude() {
            return;
        }

        let dir = temp_dir();

        // Create all ralph files
        fs::write(dir.path().join("SPEC.md"), "# Spec").unwrap();
        fs::write(dir.path().join("IMPLEMENTATION_PLAN.md"), "# Plan").unwrap();
        fs::write(dir.path().join("PROMPT.md"), "# Prompt").unwrap();

        // Error should list all files
        ralphctl()
            .current_dir(dir.path())
            .arg("init")
            .assert()
            .failure()
            .stderr(predicate::str::contains("SPEC.md"))
            .stderr(predicate::str::contains("IMPLEMENTATION_PLAN.md"))
            .stderr(predicate::str::contains("PROMPT.md"));
    }
}
