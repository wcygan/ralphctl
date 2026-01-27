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

// ========== Reverse mode file tests ==========

#[test]
fn archive_reverse_files_copies_to_archive() {
    let dir = temp_dir();

    let question_content = "# Investigation Question\n\nWhy does auth fail?";
    let investigation_content = "# Investigation Log\n\n## Hypothesis 1\n- [x] Checked auth.rs";
    let findings_content = "# Investigation Findings\n\nThe bug is in auth.rs:42";

    fs::write(dir.path().join("QUESTION.md"), question_content).unwrap();
    fs::write(dir.path().join("INVESTIGATION.md"), investigation_content).unwrap();
    fs::write(dir.path().join("FINDINGS.md"), findings_content).unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("archive")
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("Archived 3 files"));

    // Find the archive directory
    let archive_base = dir.path().join(".ralphctl").join("archive");
    let timestamp_dirs: Vec<_> = fs::read_dir(&archive_base).unwrap().collect();
    assert_eq!(timestamp_dirs.len(), 1);

    let timestamp_dir = timestamp_dirs[0].as_ref().unwrap().path();

    // Verify archived files have original content
    assert_eq!(
        fs::read_to_string(timestamp_dir.join("QUESTION.md")).unwrap(),
        question_content
    );
    assert_eq!(
        fs::read_to_string(timestamp_dir.join("INVESTIGATION.md")).unwrap(),
        investigation_content
    );
    assert_eq!(
        fs::read_to_string(timestamp_dir.join("FINDINGS.md")).unwrap(),
        findings_content
    );
}

#[test]
fn archive_reverse_files_resets_question_and_investigation() {
    let dir = temp_dir();

    fs::write(dir.path().join("QUESTION.md"), "# Original question").unwrap();
    fs::write(
        dir.path().join("INVESTIGATION.md"),
        "# Original investigation",
    )
    .unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("archive")
        .arg("--force")
        .assert()
        .success();

    // Verify files are reset to blank templates
    let question = fs::read_to_string(dir.path().join("QUESTION.md")).unwrap();
    let investigation = fs::read_to_string(dir.path().join("INVESTIGATION.md")).unwrap();

    assert!(question.contains("# Investigation Question"));
    assert!(question.contains("Describe what you want to investigate"));
    assert_eq!(investigation, "# Investigation Log\n\n");
}

#[test]
fn archive_reverse_files_deletes_findings() {
    let dir = temp_dir();

    fs::write(dir.path().join("QUESTION.md"), "# Question").unwrap();
    fs::write(dir.path().join("FINDINGS.md"), "# Findings with answer").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("archive")
        .arg("--force")
        .assert()
        .success();

    // FINDINGS.md should be deleted, not reset
    assert!(!dir.path().join("FINDINGS.md").exists());

    // QUESTION.md should be reset (still exists)
    assert!(dir.path().join("QUESTION.md").exists());
}

#[test]
fn archive_both_modes_together() {
    let dir = temp_dir();

    // Create forward mode files
    fs::write(dir.path().join("SPEC.md"), "# Forward Spec").unwrap();
    fs::write(dir.path().join("IMPLEMENTATION_PLAN.md"), "# Forward Plan").unwrap();
    // Create reverse mode files
    fs::write(dir.path().join("QUESTION.md"), "# Reverse Question").unwrap();
    fs::write(
        dir.path().join("INVESTIGATION.md"),
        "# Reverse Investigation",
    )
    .unwrap();
    fs::write(dir.path().join("FINDINGS.md"), "# Reverse Findings").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("archive")
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("Archived 5 files"));

    // Find the archive directory
    let archive_base = dir.path().join(".ralphctl").join("archive");
    let timestamp_dirs: Vec<_> = fs::read_dir(&archive_base).unwrap().collect();
    let timestamp_dir = timestamp_dirs[0].as_ref().unwrap().path();

    // Verify all files were archived
    assert!(timestamp_dir.join("SPEC.md").exists());
    assert!(timestamp_dir.join("IMPLEMENTATION_PLAN.md").exists());
    assert!(timestamp_dir.join("QUESTION.md").exists());
    assert!(timestamp_dir.join("INVESTIGATION.md").exists());
    assert!(timestamp_dir.join("FINDINGS.md").exists());

    // Verify forward files are reset
    assert_eq!(
        fs::read_to_string(dir.path().join("SPEC.md")).unwrap(),
        "# Specification\n\n"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("IMPLEMENTATION_PLAN.md")).unwrap(),
        "# Implementation Plan\n\n"
    );

    // Verify QUESTION.md and INVESTIGATION.md are reset
    assert!(fs::read_to_string(dir.path().join("QUESTION.md"))
        .unwrap()
        .contains("# Investigation Question"));
    assert_eq!(
        fs::read_to_string(dir.path().join("INVESTIGATION.md")).unwrap(),
        "# Investigation Log\n\n"
    );

    // Verify FINDINGS.md is deleted
    assert!(!dir.path().join("FINDINGS.md").exists());
}

#[test]
fn archive_reverse_excludes_reverse_prompt() {
    let dir = temp_dir();

    // REVERSE_PROMPT.md is a template, should NOT be archived
    fs::write(dir.path().join("QUESTION.md"), "# Question").unwrap();
    fs::write(
        dir.path().join("REVERSE_PROMPT.md"),
        "# Reverse Prompt Template",
    )
    .unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("archive")
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("Archived 1 file")); // Only QUESTION.md

    // Verify REVERSE_PROMPT.md was NOT archived
    let archive_base = dir.path().join(".ralphctl").join("archive");
    let timestamp_dirs: Vec<_> = fs::read_dir(&archive_base).unwrap().collect();
    let timestamp_dir = timestamp_dirs[0].as_ref().unwrap().path();

    assert!(!timestamp_dir.join("REVERSE_PROMPT.md").exists());

    // REVERSE_PROMPT.md should still exist in the original location, unchanged
    assert_eq!(
        fs::read_to_string(dir.path().join("REVERSE_PROMPT.md")).unwrap(),
        "# Reverse Prompt Template"
    );
}

#[test]
fn archive_prompt_includes_reverse_file_count() {
    let dir = temp_dir();

    fs::write(dir.path().join("QUESTION.md"), "# Question").unwrap();
    fs::write(dir.path().join("INVESTIGATION.md"), "# Investigation").unwrap();
    fs::write(dir.path().join("FINDINGS.md"), "# Findings").unwrap();

    ralphctl()
        .current_dir(dir.path())
        .arg("archive")
        .write_stdin("n\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("Archive 3 files?"));
}
