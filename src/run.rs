//! Run command implementation for ralphctl.
//!
//! Provides the core ralph loop execution logic.

use crate::{error, files};
use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

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

/// Result of running a single iteration of the claude subprocess.
#[allow(dead_code)] // Used in future iteration loop implementation
#[derive(Debug)]
pub struct IterationResult {
    /// Whether the subprocess exited successfully (exit code 0)
    pub success: bool,
    /// Exit code from the subprocess
    pub exit_code: Option<i32>,
}

/// Spawn `claude -p` as a subprocess and pipe the prompt via stdin.
///
/// Stdout and stderr are inherited, streaming directly to the terminal.
/// Returns the result of the iteration after claude completes.
#[allow(dead_code)] // Used in future iteration loop implementation
pub fn spawn_claude(prompt: &str) -> Result<IterationResult> {
    let mut child = Command::new("claude")
        .arg("-p")
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .inspect_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                error::die("claude not found in PATH");
            }
        })?;

    // Write prompt to stdin, then drop to signal EOF
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(prompt.as_bytes())?;
        // stdin is dropped here, closing the pipe
    }

    // Wait for claude to complete
    let status = child.wait()?;

    Ok(IterationResult {
        success: status.success(),
        exit_code: status.code(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;
    use tempfile::TempDir;

    // Mutex to serialize tests that change the working directory
    static DIR_MUTEX: Mutex<()> = Mutex::new(());

    fn with_temp_dir<F>(f: F)
    where
        F: FnOnce(&TempDir),
    {
        let _guard = DIR_MUTEX.lock().unwrap();
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let original_dir = env::current_dir().expect("Failed to get current dir");
        env::set_current_dir(dir.path()).expect("Failed to change to temp dir");
        f(&dir);
        // Restore original dir - ignore errors since another test might have changed it
        let _ = env::set_current_dir(original_dir);
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

    #[test]
    fn test_spawn_echo_command() {
        // Test subprocess spawning using echo instead of claude
        // This verifies the piping mechanism works correctly
        let mut child = Command::new("cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn cat");

        let test_input = "Hello from stdin";

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(test_input.as_bytes()).unwrap();
        }

        let output = child.wait_with_output().expect("Failed to wait on child");
        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout), test_input);
    }

    #[test]
    fn test_iteration_result_debug() {
        let result = IterationResult {
            success: true,
            exit_code: Some(0),
        };
        // Verify Debug trait is implemented
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("success: true"));
        assert!(debug_str.contains("exit_code: Some(0)"));
    }
}
