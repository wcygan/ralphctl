//! Run command implementation for ralphctl.
//!
//! Provides the core ralph loop execution logic.

use crate::{error, files};
use anyhow::Result;
use std::fs;
use std::path::Path;

/// Required files that must exist before running.
const REQUIRED_FILES: &[&str] = &[
    files::PROMPT_FILE,
    files::SPEC_FILE,
    files::IMPLEMENTATION_PLAN_FILE,
];

/// Validate that all required files exist before starting the loop.
pub fn validate_required_files() -> Result<()> {
    let cwd = Path::new(".");
    let missing: Vec<_> = REQUIRED_FILES
        .iter()
        .filter(|f| !cwd.join(f).exists())
        .copied()
        .collect();

    if !missing.is_empty() {
        error::die(&format!("missing required files: {}", missing.join(", ")));
    }

    Ok(())
}

/// Read the contents of PROMPT.md.
///
/// Returns the full prompt content as a string to be piped to claude.
pub fn read_prompt() -> Result<String> {
    let path = Path::new(files::PROMPT_FILE);
    if !path.exists() {
        error::die(&format!("{} not found", files::PROMPT_FILE));
    }

    let content = fs::read_to_string(path)?;
    if content.trim().is_empty() {
        error::die(&format!("{} is empty", files::PROMPT_FILE));
    }

    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    fn with_temp_dir<F>(f: F)
    where
        F: FnOnce(&TempDir),
    {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let original_dir = env::current_dir().expect("Failed to get current dir");
        env::set_current_dir(dir.path()).expect("Failed to change to temp dir");
        f(&dir);
        env::set_current_dir(original_dir).expect("Failed to restore original dir");
    }

    #[test]
    fn test_read_prompt_success() {
        with_temp_dir(|dir| {
            let prompt_content = "# Ralph Loop Prompt\n\nDo the thing.";
            fs::write(dir.path().join(files::PROMPT_FILE), prompt_content).unwrap();

            let result = read_prompt().unwrap();
            assert_eq!(result, prompt_content);
        });
    }

    #[test]
    fn test_validate_required_files_all_present() {
        with_temp_dir(|dir| {
            // Create all required files
            fs::write(dir.path().join(files::PROMPT_FILE), "prompt").unwrap();
            fs::write(dir.path().join(files::SPEC_FILE), "spec").unwrap();
            fs::write(dir.path().join(files::IMPLEMENTATION_PLAN_FILE), "plan").unwrap();

            let result = validate_required_files();
            assert!(result.is_ok());
        });
    }
}
