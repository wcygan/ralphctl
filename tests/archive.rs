//! Integration tests for the `ralphctl archive` command.

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
fn archive_no_files_succeeds() {
    let dir = temp_dir();

    ralphctl()
        .current_dir(dir.path())
        .arg("archive")
        .assert()
        .success()
        .stdout(predicate::str::contains("No archivable files found."));
}

#[test]
fn archive_force_creates_archive_directory() {
    let dir = temp_dir();

    // Create archivable file
    fs::write(dir.path().join("SPEC.md"), "# My Spec").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("archive")
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("Archived 1 file"));

    // Verify .ralphctl/archive directory exists with timestamp subdirectory
    let ralphctl_dir = dir.path().join(".ralphctl").join("archive");
    assert!(ralphctl_dir.exists());

    // Should have exactly one timestamp directory
    let entries: Vec<_> = fs::read_dir(&ralphctl_dir).unwrap().collect();
    assert_eq!(entries.len(), 1);
}

#[test]
fn archive_copies_files_to_archive() {
    let dir = temp_dir();

    let spec_content = "# My Feature Spec\n\nThis is the spec content.";
    let plan_content = "# Implementation Plan\n\n- [ ] Task 1\n- [x] Task 2";

    fs::write(dir.path().join("SPEC.md"), spec_content).unwrap();
    fs::write(dir.path().join("IMPLEMENTATION_PLAN.md"), plan_content).unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("archive")
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("Archived 2 files"));

    // Find the archive directory
    let archive_base = dir.path().join(".ralphctl").join("archive");
    let timestamp_dirs: Vec<_> = fs::read_dir(&archive_base).unwrap().collect();
    assert_eq!(timestamp_dirs.len(), 1);

    let timestamp_dir = timestamp_dirs[0].as_ref().unwrap().path();

    // Verify archived files have original content
    let archived_spec = fs::read_to_string(timestamp_dir.join("SPEC.md")).unwrap();
    let archived_plan = fs::read_to_string(timestamp_dir.join("IMPLEMENTATION_PLAN.md")).unwrap();

    assert_eq!(archived_spec, spec_content);
    assert_eq!(archived_plan, plan_content);
}

#[test]
fn archive_resets_original_files_to_blank() {
    let dir = temp_dir();

    fs::write(dir.path().join("SPEC.md"), "# Original Spec Content").unwrap();
    fs::write(dir.path().join("IMPLEMENTATION_PLAN.md"), "# Original Plan").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("archive")
        .arg("--force")
        .assert()
        .success();

    // Verify original files are now blank templates
    let spec = fs::read_to_string(dir.path().join("SPEC.md")).unwrap();
    let plan = fs::read_to_string(dir.path().join("IMPLEMENTATION_PLAN.md")).unwrap();

    assert_eq!(spec, "# Specification\n\n");
    assert_eq!(plan, "# Implementation Plan\n\n");
}

#[test]
fn archive_updates_gitignore() {
    let dir = temp_dir();

    fs::write(dir.path().join("SPEC.md"), "# Spec").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("archive")
        .arg("--force")
        .assert()
        .success();

    // Verify .gitignore contains .ralphctl
    let gitignore = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
    assert!(gitignore.lines().any(|line| line.trim() == ".ralphctl"));
}

#[test]
fn archive_creates_gitignore_if_missing() {
    let dir = temp_dir();

    fs::write(dir.path().join("SPEC.md"), "# Spec").unwrap();

    // No .gitignore exists initially
    assert!(!dir.path().join(".gitignore").exists());

    ralphctl()
        .current_dir(dir.path())
        .arg("archive")
        .arg("--force")
        .assert()
        .success();

    // .gitignore should now exist with .ralphctl
    let gitignore = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
    assert_eq!(gitignore.trim(), ".ralphctl");
}

#[test]
fn archive_does_not_duplicate_gitignore_entry() {
    let dir = temp_dir();

    // Create .gitignore with existing .ralphctl entry
    fs::write(dir.path().join(".gitignore"), ".ralphctl\ntarget/\n").unwrap();
    fs::write(dir.path().join("SPEC.md"), "# Spec").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("archive")
        .arg("--force")
        .assert()
        .success();

    // .gitignore should still have only one .ralphctl entry
    let gitignore = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
    let count = gitignore
        .lines()
        .filter(|line| line.trim() == ".ralphctl")
        .count();
    assert_eq!(count, 1);
}

#[test]
fn archive_without_force_prompts_user() {
    let dir = temp_dir();

    fs::write(dir.path().join("SPEC.md"), "# Spec").unwrap();

    // Empty input should decline
    ralphctl()
        .current_dir(dir.path())
        .arg("archive")
        .write_stdin("\n")
        .assert()
        .code(1);

    // File should still exist with original content
    let content = fs::read_to_string(dir.path().join("SPEC.md")).unwrap();
    assert_eq!(content, "# Spec");
}

#[test]
fn archive_without_force_accepts_y() {
    let dir = temp_dir();

    fs::write(dir.path().join("SPEC.md"), "# Spec").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("archive")
        .write_stdin("y\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Archived 1 file"));

    // File should be reset to blank
    let content = fs::read_to_string(dir.path().join("SPEC.md")).unwrap();
    assert_eq!(content, "# Specification\n\n");
}

#[test]
fn archive_preserves_non_archivable_files() {
    let dir = temp_dir();

    // Create archivable and non-archivable files
    fs::write(dir.path().join("SPEC.md"), "# Spec").unwrap();
    fs::write(dir.path().join("PROMPT.md"), "# Prompt").unwrap();
    fs::write(dir.path().join("ralph.log"), "log content").unwrap();
    fs::write(dir.path().join("README.md"), "# Readme").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("archive")
        .arg("--force")
        .assert()
        .success();

    // Non-archivable files should remain unchanged
    assert_eq!(
        fs::read_to_string(dir.path().join("PROMPT.md")).unwrap(),
        "# Prompt"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("ralph.log")).unwrap(),
        "log content"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("README.md")).unwrap(),
        "# Readme"
    );

    // Only archivable file should be reset
    assert_eq!(
        fs::read_to_string(dir.path().join("SPEC.md")).unwrap(),
        "# Specification\n\n"
    );
}

#[test]
fn archive_prompt_shows_file_count() {
    let dir = temp_dir();

    fs::write(dir.path().join("SPEC.md"), "# Spec").unwrap();
    fs::write(dir.path().join("IMPLEMENTATION_PLAN.md"), "# Plan").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("archive")
        .write_stdin("n\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("Archive 2 files?"));
}
