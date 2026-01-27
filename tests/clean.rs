//! Integration tests for the `ralphctl clean` command.

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

#[test]
fn clean_no_files_succeeds() {
    let dir = temp_dir();

    ralphctl()
        .current_dir(dir.path())
        .arg("clean")
        .assert()
        .success()
        .stdout(predicate::str::contains("No ralph files found."));
}

#[test]
fn clean_force_deletes_without_prompt() {
    let dir = temp_dir();

    // Create ralph files
    fs::write(dir.path().join("SPEC.md"), "# Spec").unwrap();
    fs::write(dir.path().join("IMPLEMENTATION_PLAN.md"), "# Plan").unwrap();
    fs::write(dir.path().join("PROMPT.md"), "# Prompt").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("clean")
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted 3 files."));

    // Verify files are deleted
    assert!(!dir.path().join("SPEC.md").exists());
    assert!(!dir.path().join("IMPLEMENTATION_PLAN.md").exists());
    assert!(!dir.path().join("PROMPT.md").exists());
}

#[test]
fn clean_force_single_file() {
    let dir = temp_dir();

    // Create only one ralph file
    fs::write(dir.path().join("ralph.log"), "log content").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("clean")
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted 1 file."));

    assert!(!dir.path().join("ralph.log").exists());
}

#[test]
fn clean_force_all_ralph_files() {
    let dir = temp_dir();

    // Create all ralph files
    fs::write(dir.path().join("SPEC.md"), "# Spec").unwrap();
    fs::write(dir.path().join("IMPLEMENTATION_PLAN.md"), "# Plan").unwrap();
    fs::write(dir.path().join("PROMPT.md"), "# Prompt").unwrap();
    fs::write(dir.path().join("ralph.log"), "log content").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("clean")
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted 4 files."));

    // Verify all files are deleted
    assert!(!dir.path().join("SPEC.md").exists());
    assert!(!dir.path().join("IMPLEMENTATION_PLAN.md").exists());
    assert!(!dir.path().join("PROMPT.md").exists());
    assert!(!dir.path().join("ralph.log").exists());
}

#[test]
fn clean_preserves_non_ralph_files() {
    let dir = temp_dir();

    // Create ralph file and non-ralph file
    fs::write(dir.path().join("SPEC.md"), "# Spec").unwrap();
    fs::write(dir.path().join("README.md"), "# Readme").unwrap();
    fs::write(dir.path().join("src.rs"), "fn main() {}").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("clean")
        .arg("--force")
        .assert()
        .success();

    // Verify ralph file deleted, others preserved
    assert!(!dir.path().join("SPEC.md").exists());
    assert!(dir.path().join("README.md").exists());
    assert!(dir.path().join("src.rs").exists());
}

#[test]
fn clean_without_force_declines_on_empty_input() {
    let dir = temp_dir();

    // Create ralph file
    fs::write(dir.path().join("SPEC.md"), "# Spec").unwrap();

    // Simulate empty input (just pressing Enter)
    ralphctl()
        .current_dir(dir.path())
        .arg("clean")
        .write_stdin("\n")
        .assert()
        .code(1); // Should exit with error when user declines

    // File should still exist
    assert!(dir.path().join("SPEC.md").exists());
}

#[test]
fn clean_without_force_accepts_y() {
    let dir = temp_dir();

    // Create ralph file
    fs::write(dir.path().join("SPEC.md"), "# Spec").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("clean")
        .write_stdin("y\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted 1 file."));

    // File should be deleted
    assert!(!dir.path().join("SPEC.md").exists());
}

#[test]
fn clean_without_force_accepts_yes() {
    let dir = temp_dir();

    // Create ralph file
    fs::write(dir.path().join("PROMPT.md"), "# Prompt").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("clean")
        .write_stdin("yes\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted 1 file."));

    assert!(!dir.path().join("PROMPT.md").exists());
}

#[test]
fn clean_without_force_rejects_n() {
    let dir = temp_dir();

    // Create ralph file
    fs::write(dir.path().join("SPEC.md"), "# Spec").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("clean")
        .write_stdin("n\n")
        .assert()
        .code(1);

    // File should still exist
    assert!(dir.path().join("SPEC.md").exists());
}

#[test]
fn clean_without_force_rejects_invalid_input() {
    let dir = temp_dir();

    // Create ralph file
    fs::write(dir.path().join("SPEC.md"), "# Spec").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("clean")
        .write_stdin("maybe\n")
        .assert()
        .code(1);

    // File should still exist
    assert!(dir.path().join("SPEC.md").exists());
}

#[test]
fn clean_prompt_shows_file_count() {
    let dir = temp_dir();

    // Create multiple ralph files
    fs::write(dir.path().join("SPEC.md"), "# Spec").unwrap();
    fs::write(dir.path().join("PROMPT.md"), "# Prompt").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("clean")
        .write_stdin("n\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("Delete 2 ralph files?"));
}

// ========== Reverse mode file tests ==========

#[test]
fn clean_force_deletes_reverse_files() {
    let dir = temp_dir();

    // Create reverse mode files
    fs::write(dir.path().join("QUESTION.md"), "# Question").unwrap();
    fs::write(dir.path().join("INVESTIGATION.md"), "# Investigation").unwrap();
    fs::write(dir.path().join("FINDINGS.md"), "# Findings").unwrap();
    fs::write(dir.path().join("REVERSE_PROMPT.md"), "# Prompt").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("clean")
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted 4 files."));

    // Verify all files are deleted
    assert!(!dir.path().join("QUESTION.md").exists());
    assert!(!dir.path().join("INVESTIGATION.md").exists());
    assert!(!dir.path().join("FINDINGS.md").exists());
    assert!(!dir.path().join("REVERSE_PROMPT.md").exists());
}

#[test]
fn clean_force_deletes_all_ralph_files_both_modes() {
    let dir = temp_dir();

    // Create forward mode files
    fs::write(dir.path().join("SPEC.md"), "# Spec").unwrap();
    fs::write(dir.path().join("IMPLEMENTATION_PLAN.md"), "# Plan").unwrap();
    fs::write(dir.path().join("PROMPT.md"), "# Prompt").unwrap();
    fs::write(dir.path().join("ralph.log"), "log content").unwrap();
    // Create reverse mode files
    fs::write(dir.path().join("QUESTION.md"), "# Question").unwrap();
    fs::write(dir.path().join("INVESTIGATION.md"), "# Investigation").unwrap();
    fs::write(dir.path().join("FINDINGS.md"), "# Findings").unwrap();
    fs::write(dir.path().join("REVERSE_PROMPT.md"), "# Reverse Prompt").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("clean")
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted 8 files."));

    // Verify all files are deleted
    // Forward mode
    assert!(!dir.path().join("SPEC.md").exists());
    assert!(!dir.path().join("IMPLEMENTATION_PLAN.md").exists());
    assert!(!dir.path().join("PROMPT.md").exists());
    assert!(!dir.path().join("ralph.log").exists());
    // Reverse mode
    assert!(!dir.path().join("QUESTION.md").exists());
    assert!(!dir.path().join("INVESTIGATION.md").exists());
    assert!(!dir.path().join("FINDINGS.md").exists());
    assert!(!dir.path().join("REVERSE_PROMPT.md").exists());
}

#[test]
fn clean_reverse_files_preserves_forward_files() {
    let dir = temp_dir();

    // Create only reverse mode files (no forward files)
    fs::write(dir.path().join("QUESTION.md"), "# Question").unwrap();
    fs::write(dir.path().join("INVESTIGATION.md"), "# Investigation").unwrap();

    // Create non-ralph file
    fs::write(dir.path().join("README.md"), "# Readme").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("clean")
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted 2 files."));

    // Verify reverse files deleted, non-ralph preserved
    assert!(!dir.path().join("QUESTION.md").exists());
    assert!(!dir.path().join("INVESTIGATION.md").exists());
    assert!(dir.path().join("README.md").exists());
}

#[test]
fn clean_prompt_includes_reverse_file_count() {
    let dir = temp_dir();

    // Create reverse mode files
    fs::write(dir.path().join("QUESTION.md"), "# Question").unwrap();
    fs::write(dir.path().join("INVESTIGATION.md"), "# Investigation").unwrap();
    fs::write(dir.path().join("FINDINGS.md"), "# Findings").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("clean")
        .write_stdin("n\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("Delete 3 ralph files?"));
}
