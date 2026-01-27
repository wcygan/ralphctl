//! Integration tests for the `ralphctl fetch-latest-prompt` command.

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
fn fetch_latest_prompt_creates_file() {
    let dir = temp_dir();

    ralphctl()
        .current_dir(dir.path())
        .arg("fetch-latest-prompt")
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated PROMPT.md"));

    assert!(dir.path().join("PROMPT.md").exists());
    let content = fs::read_to_string(dir.path().join("PROMPT.md")).unwrap();
    assert!(content.contains("[[RALPH:")); // Contains magic strings
}

#[test]
fn fetch_latest_prompt_overwrites_existing() {
    let dir = temp_dir();
    fs::write(dir.path().join("PROMPT.md"), "old content").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("fetch-latest-prompt")
        .assert()
        .success();

    let content = fs::read_to_string(dir.path().join("PROMPT.md")).unwrap();
    assert_ne!(content, "old content");
}

#[test]
fn fetch_latest_prompt_does_not_touch_spec() {
    let dir = temp_dir();
    let spec_content = "# My Custom Spec\n\nThis should not change.";
    fs::write(dir.path().join("SPEC.md"), spec_content).unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("fetch-latest-prompt")
        .assert()
        .success();

    // SPEC.md should remain unchanged
    let actual = fs::read_to_string(dir.path().join("SPEC.md")).unwrap();
    assert_eq!(actual, spec_content);
}

#[test]
fn fetch_latest_prompt_does_not_touch_implementation_plan() {
    let dir = temp_dir();
    let plan_content = "# My Custom Plan\n\n- [x] Task 1\n- [ ] Task 2";
    fs::write(dir.path().join("IMPLEMENTATION_PLAN.md"), plan_content).unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("fetch-latest-prompt")
        .assert()
        .success();

    // IMPLEMENTATION_PLAN.md should remain unchanged
    let actual = fs::read_to_string(dir.path().join("IMPLEMENTATION_PLAN.md")).unwrap();
    assert_eq!(actual, plan_content);
}

#[test]
fn fetch_latest_prompt_help_shows_description() {
    ralphctl()
        .arg("fetch-latest-prompt")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("PROMPT.md"))
        .stdout(predicate::str::contains("GitHub"));
}
