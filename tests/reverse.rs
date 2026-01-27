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

#[test]
fn reverse_without_args_and_no_question_file_creates_template() {
    let dir = temp_dir();

    // No QUESTION.md exists, no argument provided
    // The command should create a template and exit with code 1

    ralphctl()
        .current_dir(dir.path())
        .arg("reverse")
        .assert()
        .code(1) // error::exit::ERROR
        .stderr(predicate::str::contains("Created QUESTION.md"))
        .stderr(predicate::str::contains(
            "Edit it with your investigation question",
        ));

    // Verify QUESTION.md template was created
    let question_path = dir.path().join("QUESTION.md");
    assert!(question_path.exists(), "QUESTION.md should be created");

    let content = fs::read_to_string(&question_path).unwrap();
    assert!(
        content.contains("# Investigation Question"),
        "Template should have header"
    );
    assert!(
        content.contains("Describe what you want to investigate"),
        "Template should have placeholder text"
    );
}

#[test]
fn reverse_without_args_no_question_does_not_create_other_files() {
    let dir = temp_dir();

    // Run reverse without args and no QUESTION.md
    ralphctl()
        .current_dir(dir.path())
        .arg("reverse")
        .assert()
        .code(1);

    // Only QUESTION.md should be created, not REVERSE_PROMPT.md or ralph.log
    assert!(
        dir.path().join("QUESTION.md").exists(),
        "QUESTION.md should exist"
    );
    assert!(
        !dir.path().join("REVERSE_PROMPT.md").exists(),
        "REVERSE_PROMPT.md should NOT be created"
    );
    assert!(
        !dir.path().join("ralph.log").exists(),
        "ralph.log should NOT be created"
    );
    assert!(
        !dir.path().join("INVESTIGATION.md").exists(),
        "INVESTIGATION.md should NOT be created"
    );
}

#[test]
fn reverse_without_args_exits_before_checking_claude() {
    let dir = temp_dir();

    // Set PATH to empty so claude won't be found
    // If it checked for claude before creating template, it would error differently

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", "") // Remove PATH so claude can't be found
        .arg("reverse")
        .assert()
        .code(1) // Should exit 1 from template creation, not from missing claude
        .stderr(predicate::str::contains("Created QUESTION.md"));

    // Template should still be created
    assert!(dir.path().join("QUESTION.md").exists());
}

// ==================== Signal Tests ====================

#[test]
fn reverse_continue_signal_proceeds_to_next_iteration() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // Create mock claude that outputs CONTINUE signal
    // This should cause the loop to continue without prompting
    let mock_output = "Investigating hypothesis 1...\n[[RALPH:CONTINUE]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    // With max-iterations=2 and CONTINUE signal, should run both iterations
    // then exit with MAX_ITERATIONS code
    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Why does auth fail?")
        .arg("--max-iterations")
        .arg("2")
        .assert()
        .code(2) // MAX_ITERATIONS because CONTINUE keeps looping
        .stderr(predicate::str::contains("reached max iterations"));

    // Verify both iterations ran
    let log_content = fs::read_to_string(dir.path().join("ralph.log")).unwrap();
    assert!(
        log_content.contains("=== Iteration 1 starting ==="),
        "Iteration 1 should be logged"
    );
    assert!(
        log_content.contains("=== Iteration 2 starting ==="),
        "Iteration 2 should be logged"
    );
}

#[test]
fn reverse_continue_signal_with_whitespace() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // CONTINUE signal can have leading/trailing whitespace on its line
    let mock_output = "Investigating...\n  [[RALPH:CONTINUE]]  \n";
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
        .code(2); // Runs one iteration with CONTINUE, then hits max
}

#[test]
fn reverse_continue_shows_iteration_headers_for_all_iterations() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    let mock_output = "Working on hypothesis...\n[[RALPH:CONTINUE]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    let output = ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Why does the test fail?")
        .arg("--max-iterations")
        .arg("3")
        .assert()
        .code(2)
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        stdout.contains("=== Iteration 1 starting ==="),
        "Should show iteration 1 header"
    );
    assert!(
        stdout.contains("=== Iteration 2 starting ==="),
        "Should show iteration 2 header"
    );
    assert!(
        stdout.contains("=== Iteration 3 starting ==="),
        "Should show iteration 3 header"
    );
}

// ==================== FOUND Signal Tests ====================

#[test]
fn reverse_found_signal_exits_with_success() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // Mock claude outputs FOUND signal
    let mock_output = "Investigating the authentication flow...\n\
                       Examined src/auth.rs, found the issue.\n\
                       [[RALPH:FOUND:Bug in session token validation at auth.rs:142]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Why does authentication fail?")
        .arg("--max-iterations")
        .arg("10")
        .assert()
        .success() // Exit code 0
        .stdout(predicate::str::contains("=== Investigation complete ==="))
        .stdout(predicate::str::contains(
            "Bug in session token validation at auth.rs:142",
        ));
}

#[test]
fn reverse_found_signal_stops_loop_immediately() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // FOUND signal should stop on first iteration, even with high max-iterations
    let mock_output = "[[RALPH:FOUND:Answer found on first try]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    let output = ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Quick question")
        .arg("--max-iterations")
        .arg("100") // High limit that should never be reached
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should only have one iteration
    assert!(
        stdout.contains("=== Iteration 1 starting ==="),
        "Should show iteration 1 header"
    );
    assert!(
        !stdout.contains("=== Iteration 2 starting ==="),
        "Should NOT start iteration 2 after FOUND"
    );
}

#[test]
fn reverse_found_signal_displays_summary_message() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    let summary = "The cache invalidation bug is caused by a race condition in cache.rs:87";
    let mock_output = format!("Investigation work...\n[[RALPH:FOUND:{}]]\n", summary);
    let bin_dir = create_mock_claude(&dir, &mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Why does the cache fail?")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Found:"))
        .stdout(predicate::str::contains(summary));
}

#[test]
fn reverse_found_signal_with_special_characters_in_summary() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // Summary with special characters that might cause parsing issues
    let mock_output =
        "[[RALPH:FOUND:Error in `fn validate<T>()` at line 42 - missing trait bound]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Type error investigation")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("validate<T>()"))
        .stdout(predicate::str::contains("missing trait bound"));
}

#[test]
fn reverse_found_signal_with_whitespace() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // FOUND signal with leading/trailing whitespace on its line
    let mock_output = "Investigating...\n  [[RALPH:FOUND:The answer is 42]]  \n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("What is the answer?")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("The answer is 42"));
}

#[test]
fn reverse_found_signal_logs_to_ralph_log() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    let mock_output = "Investigation output before signal.\n[[RALPH:FOUND:Logged finding]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Log test question")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .success();

    // Verify ralph.log was created and contains the output
    let log_path = dir.path().join("ralph.log");
    assert!(log_path.exists(), "ralph.log should be created");

    let log_content = fs::read_to_string(&log_path).unwrap();
    assert!(
        log_content.contains("Investigation output before signal"),
        "Log should contain claude output"
    );
    assert!(
        log_content.contains("[[RALPH:FOUND:Logged finding]]"),
        "Log should contain the FOUND signal"
    );
}

#[test]
fn reverse_found_signal_takes_priority_over_continue() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // Both CONTINUE and FOUND in output - FOUND should win per priority rules
    // Priority: BLOCKED → FOUND → INCONCLUSIVE → CONTINUE
    let mock_output = "Working...\n[[RALPH:CONTINUE]]\nMore work...\n[[RALPH:FOUND:Found it]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Priority test")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .success() // FOUND wins, so exit 0
        .stdout(predicate::str::contains("Found it"));
}

// ==================== INCONCLUSIVE Signal Tests ====================

#[test]
fn reverse_inconclusive_signal_exits_with_code_4() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // Mock claude outputs INCONCLUSIVE signal
    let mock_output = "Investigating the authentication flow...\n\
                       Examined multiple hypotheses but no clear answer.\n\
                       [[RALPH:INCONCLUSIVE:Unable to determine root cause after examining auth.rs, session.rs, and middleware]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Why does authentication fail?")
        .arg("--max-iterations")
        .arg("10")
        .assert()
        .code(4) // Exit code 4 = INCONCLUSIVE
        .stderr(predicate::str::contains(
            "=== Investigation inconclusive ===",
        ))
        .stderr(predicate::str::contains(
            "Unable to determine root cause after examining auth.rs, session.rs, and middleware",
        ));
}

#[test]
fn reverse_inconclusive_signal_stops_loop_immediately() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // INCONCLUSIVE signal should stop on first iteration, even with high max-iterations
    let mock_output = "[[RALPH:INCONCLUSIVE:Cannot determine answer]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    let output = ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Quick question")
        .arg("--max-iterations")
        .arg("100") // High limit that should never be reached
        .assert()
        .code(4)
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should only have one iteration
    assert!(
        stdout.contains("=== Iteration 1 starting ==="),
        "Should show iteration 1 header"
    );
    assert!(
        !stdout.contains("=== Iteration 2 starting ==="),
        "Should NOT start iteration 2 after INCONCLUSIVE"
    );
}

#[test]
fn reverse_inconclusive_signal_displays_reason() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    let reason = "Exhausted all hypotheses without finding definitive evidence";
    let mock_output = format!("Investigation work...\n[[RALPH:INCONCLUSIVE:{}]]\n", reason);
    let bin_dir = create_mock_claude(&dir, &mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Why does the cache fail?")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .code(4)
        .stderr(predicate::str::contains(
            "=== Investigation inconclusive ===",
        ))
        .stderr(predicate::str::contains(reason));
}

#[test]
fn reverse_inconclusive_signal_with_special_characters_in_reason() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // Reason with special characters that might cause parsing issues
    let mock_output =
        "[[RALPH:INCONCLUSIVE:Could not trace `async fn process<T>()` - multiple code paths]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Async investigation")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .code(4)
        .stdout(predicate::str::contains("process<T>()"))
        .stdout(predicate::str::contains("multiple code paths"));
}

#[test]
fn reverse_inconclusive_signal_with_whitespace() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // INCONCLUSIVE signal with leading/trailing whitespace on its line
    let mock_output = "Investigating...\n  [[RALPH:INCONCLUSIVE:No answer found]]  \n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Whitespace test")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .code(4)
        .stdout(predicate::str::contains("No answer found"));
}

#[test]
fn reverse_inconclusive_signal_logs_to_ralph_log() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    let mock_output =
        "Investigation output before signal.\n[[RALPH:INCONCLUSIVE:Logged inconclusive]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Log test question")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .code(4);

    // Verify ralph.log was created and contains the output
    let log_path = dir.path().join("ralph.log");
    assert!(log_path.exists(), "ralph.log should be created");

    let log_content = fs::read_to_string(&log_path).unwrap();
    assert!(
        log_content.contains("Investigation output before signal"),
        "Log should contain claude output"
    );
    assert!(
        log_content.contains("[[RALPH:INCONCLUSIVE:Logged inconclusive]]"),
        "Log should contain the INCONCLUSIVE signal"
    );
}

#[test]
fn reverse_inconclusive_signal_takes_priority_over_continue() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // Both CONTINUE and INCONCLUSIVE in output - INCONCLUSIVE should win per priority rules
    // Priority: BLOCKED → FOUND → INCONCLUSIVE → CONTINUE
    let mock_output =
        "Working...\n[[RALPH:CONTINUE]]\nMore work...\n[[RALPH:INCONCLUSIVE:Giving up]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Priority test")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .code(4) // INCONCLUSIVE wins over CONTINUE
        .stdout(predicate::str::contains("Giving up"));
}

#[test]
fn reverse_found_signal_takes_priority_over_inconclusive() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // Both FOUND and INCONCLUSIVE in output - FOUND should win per priority rules
    // Priority: BLOCKED → FOUND → INCONCLUSIVE → CONTINUE
    let mock_output =
        "Working...\n[[RALPH:INCONCLUSIVE:Maybe]]\nMore work...\n[[RALPH:FOUND:Definitely found it]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Priority test")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .success() // FOUND wins over INCONCLUSIVE, so exit 0
        .stdout(predicate::str::contains("Definitely found it"));
}

#[test]
fn reverse_inconclusive_signal_with_colon_in_reason() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // Reason containing colons (should not break parsing)
    let mock_output =
        "[[RALPH:INCONCLUSIVE:Checked files: auth.rs, session.rs, none had the issue]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Colon test")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .code(4)
        .stdout(predicate::str::contains("Checked files: auth.rs"))
        .stdout(predicate::str::contains("none had the issue"));
}

#[test]
fn reverse_found_signal_with_colon_in_summary() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // Summary containing colons (should not break parsing)
    let mock_output = "[[RALPH:FOUND:Root cause: missing null check in parse_config(): line 55]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Colon test")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Root cause: missing null check"))
        .stdout(predicate::str::contains("parse_config()"));
}

// ==================== BLOCKED Signal Tests ====================

#[test]
fn reverse_blocked_signal_exits_with_code_3() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // Mock claude outputs BLOCKED signal
    let mock_output = "Investigating the authentication flow...\n\
                       Cannot proceed without access to production database.\n\
                       [[RALPH:BLOCKED:Need production database credentials to continue]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Why does authentication fail?")
        .arg("--max-iterations")
        .arg("10")
        .assert()
        .code(3) // Exit code 3 = BLOCKED
        .stderr(predicate::str::contains("blocked:"))
        .stderr(predicate::str::contains(
            "Need production database credentials to continue",
        ));
}

#[test]
fn reverse_blocked_signal_stops_loop_immediately() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // BLOCKED signal should stop on first iteration, even with high max-iterations
    let mock_output = "[[RALPH:BLOCKED:Cannot access required file]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    let output = ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Quick question")
        .arg("--max-iterations")
        .arg("100") // High limit that should never be reached
        .assert()
        .code(3)
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Should only have one iteration
    assert!(
        stdout.contains("=== Iteration 1 starting ==="),
        "Should show iteration 1 header"
    );
    assert!(
        !stdout.contains("=== Iteration 2 starting ==="),
        "Should NOT start iteration 2 after BLOCKED"
    );
}

#[test]
fn reverse_blocked_signal_displays_reason() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    let reason = "Missing API key for external service";
    let mock_output = format!("Investigation work...\n[[RALPH:BLOCKED:{}]]\n", reason);
    let bin_dir = create_mock_claude(&dir, &mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Why does the API fail?")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("blocked:"))
        .stderr(predicate::str::contains(reason));
}

#[test]
fn reverse_blocked_signal_with_special_characters_in_reason() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // Reason with special characters that might cause parsing issues
    let mock_output = "[[RALPH:BLOCKED:Cannot parse `config.json` - invalid JSON at line 42]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Config investigation")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("config.json"))
        .stderr(predicate::str::contains("invalid JSON at line 42"));
}

#[test]
fn reverse_blocked_signal_with_whitespace() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // BLOCKED signal with leading/trailing whitespace on its line
    let mock_output = "Investigating...\n  [[RALPH:BLOCKED:Access denied]]  \n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Whitespace test")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("Access denied"));
}

#[test]
fn reverse_blocked_signal_with_empty_reason() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // BLOCKED signal with empty reason should still work
    let mock_output = "[[RALPH:BLOCKED:]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Empty reason test")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .code(3) // Still exits with BLOCKED code
        .stderr(predicate::str::contains("blocked:"));
}

#[test]
fn reverse_blocked_signal_logs_to_ralph_log() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    let mock_output = "Investigation output before signal.\n[[RALPH:BLOCKED:Logged blocker]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Log test question")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .code(3);

    // Verify ralph.log was created and contains the output
    let log_path = dir.path().join("ralph.log");
    assert!(log_path.exists(), "ralph.log should be created");

    let log_content = fs::read_to_string(&log_path).unwrap();
    assert!(
        log_content.contains("Investigation output before signal"),
        "Log should contain claude output"
    );
    assert!(
        log_content.contains("[[RALPH:BLOCKED:Logged blocker]]"),
        "Log should contain the BLOCKED signal"
    );
}

#[test]
fn reverse_blocked_signal_takes_priority_over_found() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // Both FOUND and BLOCKED in output - BLOCKED should win per priority rules
    // Priority: BLOCKED → FOUND → INCONCLUSIVE → CONTINUE
    let mock_output =
        "Working...\n[[RALPH:FOUND:Answer found]]\nMore work...\n[[RALPH:BLOCKED:But actually blocked]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Priority test")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .code(3) // BLOCKED wins over FOUND
        .stderr(predicate::str::contains("But actually blocked"));
}

#[test]
fn reverse_blocked_signal_takes_priority_over_inconclusive() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // Both INCONCLUSIVE and BLOCKED in output - BLOCKED should win
    // Priority: BLOCKED → FOUND → INCONCLUSIVE → CONTINUE
    let mock_output =
        "Working...\n[[RALPH:INCONCLUSIVE:Not sure]]\n[[RALPH:BLOCKED:Cannot proceed]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Priority test")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .code(3) // BLOCKED wins over INCONCLUSIVE
        .stderr(predicate::str::contains("Cannot proceed"));
}

#[test]
fn reverse_blocked_signal_takes_priority_over_continue() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // Both CONTINUE and BLOCKED in output - BLOCKED should win
    // Priority: BLOCKED → FOUND → INCONCLUSIVE → CONTINUE
    let mock_output = "Working...\n[[RALPH:CONTINUE]]\n[[RALPH:BLOCKED:Must stop]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Priority test")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .code(3) // BLOCKED wins over CONTINUE
        .stderr(predicate::str::contains("Must stop"));
}

#[test]
fn reverse_blocked_signal_takes_priority_over_all_signals() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // All signals present - BLOCKED should win
    let mock_output = "[[RALPH:CONTINUE]]\n[[RALPH:FOUND:Found]]\n[[RALPH:INCONCLUSIVE:Maybe]]\n[[RALPH:BLOCKED:Highest priority]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("All signals test")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("Highest priority"));
}

#[test]
fn reverse_blocked_signal_with_colon_in_reason() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // Reason containing colons (should not break parsing)
    let mock_output = "[[RALPH:BLOCKED:Error: file not found: /path/to/config.yaml]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Colon test")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("Error: file not found"))
        .stderr(predicate::str::contains("/path/to/config.yaml"));
}

// ==================== Max Iterations Tests ====================

#[test]
fn reverse_max_iterations_reached_exits_with_code_2() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // Mock claude outputs CONTINUE signal, which keeps the loop going
    // until max iterations is reached
    let mock_output = "Still investigating hypothesis...\n[[RALPH:CONTINUE]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Why does the test fail?")
        .arg("--max-iterations")
        .arg("3")
        .assert()
        .code(2) // Exit code 2 = MAX_ITERATIONS
        .stderr(predicate::str::contains("reached max iterations"));
}

#[test]
fn reverse_max_iterations_runs_exact_count() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    let mock_output = "Investigating...\n[[RALPH:CONTINUE]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    // Run with max-iterations=5
    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Test max iterations count")
        .arg("--max-iterations")
        .arg("5")
        .assert()
        .code(2);

    // Verify exactly 5 iterations ran
    let log_content = fs::read_to_string(dir.path().join("ralph.log")).unwrap();
    assert!(
        log_content.contains("=== Iteration 1 starting ==="),
        "Iteration 1 should be logged"
    );
    assert!(
        log_content.contains("=== Iteration 2 starting ==="),
        "Iteration 2 should be logged"
    );
    assert!(
        log_content.contains("=== Iteration 3 starting ==="),
        "Iteration 3 should be logged"
    );
    assert!(
        log_content.contains("=== Iteration 4 starting ==="),
        "Iteration 4 should be logged"
    );
    assert!(
        log_content.contains("=== Iteration 5 starting ==="),
        "Iteration 5 should be logged"
    );
    assert!(
        !log_content.contains("=== Iteration 6 starting ==="),
        "Iteration 6 should NOT be logged (max is 5)"
    );
}

#[test]
fn reverse_max_iterations_default_is_100() {
    // Verify the default max-iterations is documented correctly in help
    ralphctl()
        .arg("reverse")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("100")); // Default value shown in help
}

#[test]
fn reverse_max_iterations_one_runs_single_iteration() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // With max-iterations=1 and CONTINUE signal, should run exactly one iteration
    let mock_output = "Single iteration work.\n[[RALPH:CONTINUE]]\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    let output = ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("Single iteration test")
        .arg("--max-iterations")
        .arg("1")
        .assert()
        .code(2)
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        stdout.contains("=== Iteration 1 starting ==="),
        "Should run iteration 1"
    );
    assert!(
        !stdout.contains("=== Iteration 2 starting ==="),
        "Should NOT run iteration 2"
    );
}

#[test]
fn reverse_max_iterations_with_no_signal_prompts_then_stops() {
    let dir = temp_dir();
    setup_reverse_prompt_cache(&dir);

    // Mock claude outputs content without any signal
    // This should trigger the no-signal prompt
    let mock_output = "Investigation work without signal.\n";
    let bin_dir = create_mock_claude(&dir, mock_output);

    let path = format!("{}:/usr/bin", bin_dir.display());

    // When prompted, user stops (s)
    ralphctl()
        .current_dir(dir.path())
        .env("PATH", &path)
        .env("HOME", dir.path())
        .arg("reverse")
        .arg("No signal test")
        .arg("--max-iterations")
        .arg("1")
        .write_stdin("s\n") // Stop when prompted
        .assert()
        .success()
        .stdout(predicate::str::contains("Stopped by user"));
}
