//! Integration tests for the `ralphctl reverse` command.
//!
//! These tests use mock scripts to simulate claude CLI output, allowing us to
//! test the reverse command's behavior without requiring the actual claude binary.

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

/// Create a mock REVERSE_PROMPT.md in the cache directory.
///
/// This prevents the test from needing network access to fetch the template.
/// On macOS, the cache is ~/Library/Caches/ralphctl/templates/
/// On Linux, it would be ~/.cache/ralphctl/templates/
fn setup_reverse_prompt_cache(dir: &TempDir) {
    // macOS uses ~/Library/Caches, Linux uses ~/.cache
    // Since we're setting HOME to the temp dir, we need to create the right structure
    #[cfg(target_os = "macos")]
    let cache_dir = dir.path().join("Library/Caches/ralphctl/templates");
    #[cfg(not(target_os = "macos"))]
    let cache_dir = dir.path().join(".cache/ralphctl/templates");

    fs::create_dir_all(&cache_dir).unwrap();
    fs::write(
        cache_dir.join("REVERSE_PROMPT.md"),
        "# Reverse Prompt\n\nInvestigate the codebase.",
    )
    .unwrap();
}

// ==================== Happy Path Tests ====================

#[test]
fn reverse_with_question_argument_creates_question_file_and_runs() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // Create mock claude that outputs FOUND signal
    let mock_output =
        "Investigating...\nFound the issue.\n[[RALPH:FOUND:The bug is in auth.rs:42]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    // Set up environment: mock claude in PATH, HOME pointing to temp dir for cache
    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Why does authentication fail?")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Investigation complete"))
        .stdout(predicate::str::contains("The bug is in auth.rs:42"));

    // Verify QUESTION.md was created with the question
    let question_content = fs::read_to_string(dir.path().join("QUESTION.md")).unwrap();
    assert!(question_content.contains("# Investigation Question"));
    assert!(question_content.contains("Why does authentication fail?"));
}

#[test]
fn reverse_with_question_prints_iteration_header() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    let mock_output = "Investigating...\n[[RALPH:FOUND:answer]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Test question")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Iteration 1 starting ==="));
}

#[test]
fn reverse_creates_ralph_log() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    let mock_output = "Investigation output.\n[[RALPH:FOUND:answer]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Test question")
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
        log_content.contains("Investigation output"),
        "Log should contain claude output"
    );
}

#[test]
fn reverse_writes_reverse_prompt_file() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    let mock_output = "[[RALPH:FOUND:answer]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Test question")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .success();

    // Verify REVERSE_PROMPT.md was written to current directory
    let prompt_path = dir.path().join("REVERSE_PROMPT.md");
    assert!(
        prompt_path.exists(),
        "REVERSE_PROMPT.md should be created in working directory"
    );
}

#[test]
fn reverse_with_long_question() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    let mock_output = "[[RALPH:FOUND:answer]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    let long_question = "Why does the authentication flow fail for OAuth users when they try to login through the mobile app on iOS devices running version 14.0 or higher?";

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg(long_question)
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .success();

    let question_content = fs::read_to_string(dir.path().join("QUESTION.md")).unwrap();
    assert!(question_content.contains(long_question));
}

#[test]
fn reverse_with_special_characters_in_question() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    let mock_output = "[[RALPH:FOUND:found it]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    let special_question = "Why does `fn foo<T>()` fail with error \"E0277\"?";

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg(special_question)
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .success();

    let question_content = fs::read_to_string(dir.path().join("QUESTION.md")).unwrap();
    assert!(question_content.contains(special_question));
}

#[test]
fn reverse_help_shows_all_flags() {
    ralphctl()
        .arg("reverse")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--max-iterations"))
        .stdout(predicate::str::contains("--pause"))
        .stdout(predicate::str::contains("--model"))
        .stdout(predicate::str::contains("QUESTION"));
}

#[test]
fn reverse_help_shows_exit_codes() {
    ralphctl()
        .arg("reverse")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("EXIT CODES"))
        .stdout(predicate::str::contains("Found"))
        .stdout(predicate::str::contains("Blocked"))
        .stdout(predicate::str::contains("Inconclusive"));
}

// ==================== No-Argument Behavior Tests ====================

#[test]
fn reverse_without_args_uses_existing_question_file() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // Pre-create QUESTION.md with an existing question
    let question_content = r#"# Investigation Question

Why does the cache invalidation fail on concurrent updates?

## Context (Optional)

The issue appears in production with high traffic.
"#;
    fs::write(dir.path().join("QUESTION.md"), question_content).unwrap();

    // Create mock claude that outputs FOUND signal
    let mock_output =
        "Reading QUESTION.md...\nInvestigating cache...\n[[RALPH:FOUND:Race condition in cache.rs]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    // Run reverse without question argument
    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Iteration 1 starting ==="))
        .stdout(predicate::str::contains("Investigation complete"))
        .stdout(predicate::str::contains("Race condition in cache.rs"));

    // Verify QUESTION.md was NOT overwritten (still has original content)
    let final_question = fs::read_to_string(dir.path().join("QUESTION.md")).unwrap();
    assert!(
        final_question.contains("cache invalidation fail on concurrent updates"),
        "QUESTION.md should retain original content"
    );
    assert!(
        final_question.contains("Context (Optional)"),
        "QUESTION.md should retain optional context section"
    );
}

#[test]
fn reverse_without_args_preserves_question_context() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // Create QUESTION.md with detailed context
    let question_content = r#"# Investigation Question

How does the payment processing handle retries?

## Context (Optional)

We're seeing duplicate charges in production. The retry logic was added in commit abc123.
Relevant files: src/payment.rs, src/stripe_client.rs
"#;
    fs::write(dir.path().join("QUESTION.md"), question_content).unwrap();

    let mock_output = "[[RALPH:FOUND:Retry logic lacks idempotency key]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .success();

    // Verify the full context is preserved
    let final_question = fs::read_to_string(dir.path().join("QUESTION.md")).unwrap();
    assert!(final_question.contains("duplicate charges in production"));
    assert!(final_question.contains("commit abc123"));
    assert!(final_question.contains("src/payment.rs"));
}
