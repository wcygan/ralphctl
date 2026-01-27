//! Ralph file detection and management utilities.
//!
//! Provides functions for locating and managing ralph loop files.

#![allow(dead_code)] // Utilities for clean and init commands

use std::path::{Path, PathBuf};

/// The canonical ralph file names (forward mode).
pub const SPEC_FILE: &str = "SPEC.md";
pub const IMPLEMENTATION_PLAN_FILE: &str = "IMPLEMENTATION_PLAN.md";
pub const PROMPT_FILE: &str = "PROMPT.md";
pub const LOG_FILE: &str = "ralph.log";

/// Reverse mode file names.
pub const QUESTION_FILE: &str = "QUESTION.md";
pub const INVESTIGATION_FILE: &str = "INVESTIGATION.md";
pub const FINDINGS_FILE: &str = "FINDINGS.md";
pub const REVERSE_PROMPT_FILE: &str = "REVERSE_PROMPT.md";

/// All forward mode ralph files that can be created/cleaned.
pub const RALPH_FILES: &[&str] = &[SPEC_FILE, IMPLEMENTATION_PLAN_FILE, PROMPT_FILE, LOG_FILE];

/// All reverse mode ralph files that can be created/cleaned.
pub const REVERSE_FILES: &[&str] = &[
    QUESTION_FILE,
    INVESTIGATION_FILE,
    FINDINGS_FILE,
    REVERSE_PROMPT_FILE,
];

/// Files that are archived (stateful files, not templates or logs).
pub const ARCHIVABLE_FILES: &[&str] = &[SPEC_FILE, IMPLEMENTATION_PLAN_FILE];

/// Reverse mode files that are archived (stateful files, not template).
/// Excludes REVERSE_PROMPT.md as it's a template fetched from GitHub.
pub const ARCHIVABLE_REVERSE_FILES: &[&str] = &[QUESTION_FILE, INVESTIGATION_FILE, FINDINGS_FILE];

/// The ralphctl directory for storing archives and other data.
pub const RALPHCTL_DIR: &str = ".ralphctl";

/// The archive subdirectory within .ralphctl.
pub const ARCHIVE_DIR: &str = "archive";

/// Find all ralph files that exist in the given directory.
///
/// Returns a list of paths to existing ralph files.
pub fn find_existing_ralph_files(dir: &Path) -> Vec<PathBuf> {
    RALPH_FILES
        .iter()
        .map(|name| dir.join(name))
        .filter(|path| path.exists())
        .collect()
}

/// Check if any ralph files exist in the given directory.
pub fn any_ralph_files_exist(dir: &Path) -> bool {
    RALPH_FILES.iter().any(|name| dir.join(name).exists())
}

/// Find all reverse mode ralph files that exist in the given directory.
///
/// Returns a list of paths to existing reverse mode files.
pub fn find_existing_reverse_files(dir: &Path) -> Vec<PathBuf> {
    REVERSE_FILES
        .iter()
        .map(|name| dir.join(name))
        .filter(|path| path.exists())
        .collect()
}

/// Check if any reverse mode files exist in the given directory.
pub fn any_reverse_files_exist(dir: &Path) -> bool {
    REVERSE_FILES.iter().any(|name| dir.join(name).exists())
}

/// Find archivable files that exist in the given directory.
///
/// Returns a list of paths to existing archivable files.
pub fn find_archivable_files(dir: &Path) -> Vec<PathBuf> {
    ARCHIVABLE_FILES
        .iter()
        .map(|name| dir.join(name))
        .filter(|path| path.exists())
        .collect()
}

/// Find archivable reverse mode files that exist in the given directory.
///
/// Returns a list of paths to existing archivable reverse mode files.
/// Excludes REVERSE_PROMPT.md as it's a template, not stateful.
pub fn find_archivable_reverse_files(dir: &Path) -> Vec<PathBuf> {
    ARCHIVABLE_REVERSE_FILES
        .iter()
        .map(|name| dir.join(name))
        .filter(|path| path.exists())
        .collect()
}

/// Get the base archive directory path (.ralphctl/archive).
pub fn archive_base_dir(dir: &Path) -> PathBuf {
    dir.join(RALPHCTL_DIR).join(ARCHIVE_DIR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_temp_dir() -> TempDir {
        tempfile::tempdir().expect("Failed to create temp dir")
    }

    #[test]
    fn test_find_existing_no_files() {
        let dir = create_temp_dir();
        let found = find_existing_ralph_files(dir.path());
        assert!(found.is_empty());
    }

    #[test]
    fn test_find_existing_some_files() {
        let dir = create_temp_dir();

        // Create only some ralph files
        fs::write(dir.path().join(SPEC_FILE), "# Spec").unwrap();
        fs::write(dir.path().join(PROMPT_FILE), "# Prompt").unwrap();

        let found = find_existing_ralph_files(dir.path());
        assert_eq!(found.len(), 2);
        assert!(found.iter().any(|p| p.ends_with(SPEC_FILE)));
        assert!(found.iter().any(|p| p.ends_with(PROMPT_FILE)));
    }

    #[test]
    fn test_find_existing_all_files() {
        let dir = create_temp_dir();

        // Create all ralph files
        for name in RALPH_FILES {
            fs::write(dir.path().join(name), "content").unwrap();
        }

        let found = find_existing_ralph_files(dir.path());
        assert_eq!(found.len(), RALPH_FILES.len());
    }

    #[test]
    fn test_any_ralph_files_exist_false() {
        let dir = create_temp_dir();
        assert!(!any_ralph_files_exist(dir.path()));
    }

    #[test]
    fn test_any_ralph_files_exist_true() {
        let dir = create_temp_dir();
        fs::write(dir.path().join(LOG_FILE), "log").unwrap();
        assert!(any_ralph_files_exist(dir.path()));
    }

    #[test]
    fn test_ralph_files_constant_completeness() {
        // Verify all expected files are in the constant
        assert!(RALPH_FILES.contains(&SPEC_FILE));
        assert!(RALPH_FILES.contains(&IMPLEMENTATION_PLAN_FILE));
        assert!(RALPH_FILES.contains(&PROMPT_FILE));
        assert!(RALPH_FILES.contains(&LOG_FILE));
        assert_eq!(RALPH_FILES.len(), 4);
    }

    #[test]
    fn test_archivable_files_constant() {
        assert!(ARCHIVABLE_FILES.contains(&SPEC_FILE));
        assert!(ARCHIVABLE_FILES.contains(&IMPLEMENTATION_PLAN_FILE));
        assert_eq!(ARCHIVABLE_FILES.len(), 2);
        // PROMPT.md and ralph.log are NOT archivable
        assert!(!ARCHIVABLE_FILES.contains(&PROMPT_FILE));
        assert!(!ARCHIVABLE_FILES.contains(&LOG_FILE));
    }

    #[test]
    fn test_find_archivable_files_empty() {
        let dir = create_temp_dir();
        let found = find_archivable_files(dir.path());
        assert!(found.is_empty());
    }

    #[test]
    fn test_find_archivable_files_only_archivable() {
        let dir = create_temp_dir();

        // Create archivable files
        fs::write(dir.path().join(SPEC_FILE), "# Spec").unwrap();
        fs::write(dir.path().join(IMPLEMENTATION_PLAN_FILE), "# Plan").unwrap();
        // Create non-archivable file
        fs::write(dir.path().join(PROMPT_FILE), "# Prompt").unwrap();

        let found = find_archivable_files(dir.path());
        assert_eq!(found.len(), 2);
        assert!(found.iter().any(|p| p.ends_with(SPEC_FILE)));
        assert!(found.iter().any(|p| p.ends_with(IMPLEMENTATION_PLAN_FILE)));
        // PROMPT.md should not be in the list
        assert!(!found.iter().any(|p| p.ends_with(PROMPT_FILE)));
    }

    #[test]
    fn test_archive_base_dir() {
        let dir = create_temp_dir();
        let archive_dir = archive_base_dir(dir.path());
        assert!(archive_dir.ends_with(".ralphctl/archive"));
    }

    // Reverse mode file tests

    #[test]
    fn test_reverse_files_constant_completeness() {
        assert!(REVERSE_FILES.contains(&QUESTION_FILE));
        assert!(REVERSE_FILES.contains(&INVESTIGATION_FILE));
        assert!(REVERSE_FILES.contains(&FINDINGS_FILE));
        assert!(REVERSE_FILES.contains(&REVERSE_PROMPT_FILE));
        assert_eq!(REVERSE_FILES.len(), 4);
    }

    #[test]
    fn test_find_existing_reverse_files_empty() {
        let dir = create_temp_dir();
        let found = find_existing_reverse_files(dir.path());
        assert!(found.is_empty());
    }

    #[test]
    fn test_find_existing_reverse_files_some() {
        let dir = create_temp_dir();

        // Create only some reverse files
        fs::write(dir.path().join(QUESTION_FILE), "# Question").unwrap();
        fs::write(dir.path().join(INVESTIGATION_FILE), "# Investigation").unwrap();

        let found = find_existing_reverse_files(dir.path());
        assert_eq!(found.len(), 2);
        assert!(found.iter().any(|p| p.ends_with(QUESTION_FILE)));
        assert!(found.iter().any(|p| p.ends_with(INVESTIGATION_FILE)));
    }

    #[test]
    fn test_find_existing_reverse_files_all() {
        let dir = create_temp_dir();

        // Create all reverse files
        for name in REVERSE_FILES {
            fs::write(dir.path().join(name), "content").unwrap();
        }

        let found = find_existing_reverse_files(dir.path());
        assert_eq!(found.len(), REVERSE_FILES.len());
    }

    #[test]
    fn test_any_reverse_files_exist_false() {
        let dir = create_temp_dir();
        assert!(!any_reverse_files_exist(dir.path()));
    }

    #[test]
    fn test_any_reverse_files_exist_true() {
        let dir = create_temp_dir();
        fs::write(dir.path().join(QUESTION_FILE), "# Question").unwrap();
        assert!(any_reverse_files_exist(dir.path()));
    }

    #[test]
    fn test_reverse_files_independent_from_forward() {
        let dir = create_temp_dir();

        // Create only forward mode files
        fs::write(dir.path().join(SPEC_FILE), "# Spec").unwrap();
        fs::write(dir.path().join(PROMPT_FILE), "# Prompt").unwrap();

        // Reverse file discovery should find nothing
        let reverse_found = find_existing_reverse_files(dir.path());
        assert!(reverse_found.is_empty());
        assert!(!any_reverse_files_exist(dir.path()));

        // But forward file discovery should find them
        let forward_found = find_existing_ralph_files(dir.path());
        assert_eq!(forward_found.len(), 2);
        assert!(any_ralph_files_exist(dir.path()));
    }

    // Archivable reverse files tests

    #[test]
    fn test_archivable_reverse_files_constant() {
        assert!(ARCHIVABLE_REVERSE_FILES.contains(&QUESTION_FILE));
        assert!(ARCHIVABLE_REVERSE_FILES.contains(&INVESTIGATION_FILE));
        assert!(ARCHIVABLE_REVERSE_FILES.contains(&FINDINGS_FILE));
        assert_eq!(ARCHIVABLE_REVERSE_FILES.len(), 3);
        // REVERSE_PROMPT.md is NOT archivable (it's a template)
        assert!(!ARCHIVABLE_REVERSE_FILES.contains(&REVERSE_PROMPT_FILE));
    }

    #[test]
    fn test_find_archivable_reverse_files_empty() {
        let dir = create_temp_dir();
        let found = find_archivable_reverse_files(dir.path());
        assert!(found.is_empty());
    }

    #[test]
    fn test_find_archivable_reverse_files_only_archivable() {
        let dir = create_temp_dir();

        // Create archivable reverse files
        fs::write(dir.path().join(QUESTION_FILE), "# Question").unwrap();
        fs::write(dir.path().join(INVESTIGATION_FILE), "# Investigation").unwrap();
        fs::write(dir.path().join(FINDINGS_FILE), "# Findings").unwrap();
        // Create non-archivable reverse file (template)
        fs::write(dir.path().join(REVERSE_PROMPT_FILE), "# Prompt").unwrap();

        let found = find_archivable_reverse_files(dir.path());
        assert_eq!(found.len(), 3);
        assert!(found.iter().any(|p| p.ends_with(QUESTION_FILE)));
        assert!(found.iter().any(|p| p.ends_with(INVESTIGATION_FILE)));
        assert!(found.iter().any(|p| p.ends_with(FINDINGS_FILE)));
        // REVERSE_PROMPT.md should not be in the list
        assert!(!found.iter().any(|p| p.ends_with(REVERSE_PROMPT_FILE)));
    }

    #[test]
    fn test_find_archivable_reverse_files_partial() {
        let dir = create_temp_dir();

        // Create only some archivable reverse files
        fs::write(dir.path().join(QUESTION_FILE), "# Question").unwrap();
        fs::write(dir.path().join(INVESTIGATION_FILE), "# Investigation").unwrap();
        // No FINDINGS.md - investigation not complete

        let found = find_archivable_reverse_files(dir.path());
        assert_eq!(found.len(), 2);
        assert!(found.iter().any(|p| p.ends_with(QUESTION_FILE)));
        assert!(found.iter().any(|p| p.ends_with(INVESTIGATION_FILE)));
    }
}
