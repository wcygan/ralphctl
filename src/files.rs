//! Ralph file detection and management utilities.
//!
//! Provides functions for locating and managing ralph loop files.

#![allow(dead_code)] // Utilities for clean and init commands

use std::path::{Path, PathBuf};

/// The canonical ralph file names.
pub const SPEC_FILE: &str = "SPEC.md";
pub const IMPLEMENTATION_PLAN_FILE: &str = "IMPLEMENTATION_PLAN.md";
pub const PROMPT_FILE: &str = "PROMPT.md";
pub const LOG_FILE: &str = "ralph.log";

/// All ralph files that can be created/cleaned.
pub const RALPH_FILES: &[&str] = &[SPEC_FILE, IMPLEMENTATION_PLAN_FILE, PROMPT_FILE, LOG_FILE];

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
}
