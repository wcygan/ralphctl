//! Integration tests for the `ralphctl run` command.
//!
//! These tests use mock scripts to simulate claude CLI output, allowing us to
//! test the run command's behavior without requiring the actual claude binary.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

/// Get a command for ralphctl.
fn ralphctl() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("ralphctl"))
}

/// Create a temporary directory for testing.
fn temp_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

/// Create a mock claude script that outputs the given content.
///
/// Returns the path to the directory containing the mock script.
fn create_mock_claude(dir: &TempDir, output: &str) -> std::path::PathBuf {
    let bin_dir = dir.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let script_path = bin_dir.join("claude");
    // Use printf with double quotes - escape special characters appropriately
    // For double-quoted strings in shell: escape \, $, `, ", and newlines
    let escaped = output
        .replace('\\', "\\\\")
        .replace('$', "\\$")
        .replace('`', "\\`")
        .replace('"', "\\\"")
        .replace('%', "%%")
        .replace('\n', "\\n");
    let script_content = format!("#!/bin/sh\nprintf \"{}\"", escaped);

    fs::write(&script_path, script_content).unwrap();

    // Make the script executable
    let mut perms = fs::metadata(&script_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script_path, perms).unwrap();

    bin_dir
}

/// Create required ralph files in the given directory.
fn create_ralph_files(dir: &TempDir) {
    fs::write(
        dir.path().join("PROMPT.md"),
        "# Test Prompt\n\nDo the task.",
    )
    .unwrap();
    fs::write(
        dir.path().join("SPEC.md"),
        "# Test Spec\n\nProject specification.",
    )
    .unwrap();
    fs::write(
        dir.path().join("IMPLEMENTATION_PLAN.md"),
        "# Plan\n\n- [ ] Task 1\n- [ ] Task 2\n",
    )
    .unwrap();
}

#[test]
fn run_fails_without_required_files() {
    let dir = temp_dir();

    // No ralph files created - should fail
    ralphctl()
        .current_dir(dir.path())
        .arg("run")
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing required files"));
}

#[test]
fn run_fails_without_prompt_md() {
    let dir = temp_dir();

    // Create only SPEC.md and IMPLEMENTATION_PLAN.md
    fs::write(dir.path().join("SPEC.md"), "# Spec").unwrap();
    fs::write(dir.path().join("IMPLEMENTATION_PLAN.md"), "# Plan").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("run")
        .assert()
        .failure()
        .stderr(predicate::str::contains("PROMPT.md"));
}

#[test]
fn run_fails_without_spec_md() {
    let dir = temp_dir();

    // Create only PROMPT.md and IMPLEMENTATION_PLAN.md
    fs::write(dir.path().join("PROMPT.md"), "# Prompt").unwrap();
    fs::write(dir.path().join("IMPLEMENTATION_PLAN.md"), "# Plan").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("run")
        .assert()
        .failure()
        .stderr(predicate::str::contains("SPEC.md"));
}

#[test]
fn run_fails_without_implementation_plan() {
    let dir = temp_dir();

    // Create only PROMPT.md and SPEC.md
    fs::write(dir.path().join("PROMPT.md"), "# Prompt").unwrap();
    fs::write(dir.path().join("SPEC.md"), "# Spec").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("run")
        .assert()
        .failure()
        .stderr(predicate::str::contains("IMPLEMENTATION_PLAN.md"));
}

#[test]
fn run_detects_done_signal_and_exits_success() {
    let dir = temp_dir();
    create_ralph_files(&dir);

    // Create mock claude that outputs DONE signal
    let mock_output = "Completed task 1.\n[[RALPH:DONE]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    // Include /usr/bin for basic Unix utilities
    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .arg("run")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Loop complete"));
}

#[test]
fn run_detects_blocked_signal_and_exits() {
    let dir = temp_dir();
    create_ralph_files(&dir);

    // Create mock claude that outputs BLOCKED signal
    let mock_output = "Cannot proceed.\n[[RALPH:BLOCKED:missing API key]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .arg("run")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .code(3) // BLOCKED exit code
        .stderr(predicate::str::contains("blocked: missing API key"));
}

#[test]
fn run_prints_iteration_header() {
    let dir = temp_dir();
    create_ralph_files(&dir);

    let mock_output = "Working on task.\n[[RALPH:DONE]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .arg("run")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Iteration 1 starting ==="));
}

#[test]
fn run_creates_ralph_log() {
    let dir = temp_dir();
    create_ralph_files(&dir);

    let mock_output = "Task output here.\n[[RALPH:DONE]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .arg("run")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .success();

    // Verify ralph.log was created
    let log_path = dir.path().join("ralph.log");
    assert!(log_path.exists(), "ralph.log should be created");

    let log_content = fs::read_to_string(&log_path).unwrap();
    assert!(
        log_content.contains("=== Iteration 1 starting ==="),
        "Log should contain iteration header"
    );
    assert!(
        log_content.contains("Task output here"),
        "Log should contain claude output"
    );
}

#[test]
fn run_respects_max_iterations() {
    let dir = temp_dir();
    create_ralph_files(&dir);

    // Create mock claude that never outputs DONE
    let mock_output = "Still working...\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .arg("run")
        .arg("--max-iterations")
        .arg("2")
        .assert()
        .code(2) // MAX_ITERATIONS exit code
        .stderr(predicate::str::contains("reached max iterations"));
}

#[test]
fn run_logs_multiple_iterations() {
    let dir = temp_dir();
    create_ralph_files(&dir);

    // Create mock claude that outputs different content each time
    // Note: This simple mock outputs the same thing, but we verify logging works
    let mock_output = "Iteration output.\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .arg("run")
        .arg("--max-iterations")
        .arg("2")
        .assert()
        .code(2); // Exits with MAX_ITERATIONS

    let log_content = fs::read_to_string(dir.path().join("ralph.log")).unwrap();
    assert!(
        log_content.contains("=== Iteration 1 starting ==="),
        "Log should contain iteration 1 header"
    );
    assert!(
        log_content.contains("=== Iteration 2 starting ==="),
        "Log should contain iteration 2 header"
    );
}

#[test]
fn run_help_shows_max_iterations_flag() {
    ralphctl()
        .arg("run")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--max-iterations"));
}

#[test]
fn run_help_shows_pause_flag() {
    ralphctl()
        .arg("run")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--pause"));
}

#[test]
fn run_help_shows_model_flag() {
    ralphctl()
        .arg("run")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--model"));
}

#[test]
fn run_fails_when_claude_not_found() {
    let dir = temp_dir();
    create_ralph_files(&dir);

    // Set PATH to exclude claude
    ralphctl()
        .current_dir(dir.path())
        .env("PATH", "/usr/bin")
        .arg("run")
        .assert()
        .failure()
        .stderr(predicate::str::contains("claude not found in PATH"));
}

#[test]
fn run_empty_blocked_reason() {
    let dir = temp_dir();
    create_ralph_files(&dir);

    // Create mock claude that outputs BLOCKED with empty reason
    let mock_output = "[[RALPH:BLOCKED:]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .arg("run")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .code(3) // BLOCKED exit code
        .stderr(predicate::str::contains("blocked:"));
}

#[test]
fn run_done_signal_rejects_inline_mention() {
    let dir = temp_dir();
    create_ralph_files(&dir);

    // DONE signal must be on its own line - inline mentions are rejected
    // to prevent false positives when Claude discusses the marker
    let mock_output = "Some text [[RALPH:DONE]] more text\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .arg("run")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .code(2) // MAX_ITERATIONS because DONE was not detected
        .stderr(predicate::str::contains("max iterations"));
}

#[test]
fn run_done_signal_with_whitespace() {
    let dir = temp_dir();
    create_ralph_files(&dir);

    // DONE signal can have leading/trailing whitespace on its line
    let mock_output = "Working...\n  [[RALPH:DONE]]  \nExtra output\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .arg("run")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Loop complete"));
}

#[test]
fn run_blocked_with_special_characters() {
    let dir = temp_dir();
    create_ralph_files(&dir);

    // Reason can contain various characters
    let mock_output = "[[RALPH:BLOCKED:can't find file: /path/to/missing.txt]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .arg("run")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .code(3)
        .stderr(predicate::str::contains(
            "blocked: can't find file: /path/to/missing.txt",
        ));
}

#[test]
fn run_handles_mock_that_ignores_stdin() {
    // Test that ralphctl handles subprocesses that don't read stdin (triggers EPIPE)
    // This is what caused the original CI failure - mock scripts using printf
    // exit before reading the piped PROMPT.md content
    let dir = temp_dir();
    create_ralph_files(&dir);

    // Create mock that outputs DONE without reading stdin
    let mock_output = "[[RALPH:DONE]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .arg("run")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Loop complete"));
}

#[test]
fn run_handles_large_prompt_with_fast_exit() {
    // Stress test: large PROMPT.md with mock that exits immediately
    // This maximizes the chance of EPIPE occurring
    let dir = temp_dir();

    // Create a large prompt file
    let large_prompt = format!(
        "# Large Prompt\n\n{}\n",
        "This is a line of prompt content.\n".repeat(1000)
    );
    fs::write(dir.path().join("PROMPT.md"), &large_prompt).unwrap();
    fs::write(dir.path().join("SPEC.md"), "# Spec").unwrap();
    fs::write(dir.path().join("IMPLEMENTATION_PLAN.md"), "# Plan\n- [ ] Task").unwrap();

    let mock_output = "[[RALPH:DONE]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .arg("run")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .success();
}
