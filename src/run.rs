//! Run command implementation for ralphctl.
//!
//! Provides the core ralph loop execution logic.

use crate::{error, files};
use anyhow::Result;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;

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
    /// Captured stdout output for magic string detection
    pub stdout: String,
    /// Captured stderr output
    pub stderr: String,
}

/// Spawn `claude -p` as a subprocess and pipe the prompt via stdin.
///
/// Streams stdout and stderr to the terminal in real-time while also
/// capturing the output for magic string detection.
/// Returns the result of the iteration after claude completes.
#[allow(dead_code)] // Used in future iteration loop implementation
pub fn spawn_claude(prompt: &str) -> Result<IterationResult> {
    let mut child = Command::new("claude")
        .arg("-p")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
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

    // Take ownership of stdout and stderr for streaming
    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();

    // Spawn thread to stream and capture stdout
    let stdout_handle = thread::spawn(move || stream_and_capture(stdout_pipe, io::stdout()));

    // Spawn thread to stream and capture stderr
    let stderr_handle = thread::spawn(move || stream_and_capture(stderr_pipe, io::stderr()));

    // Wait for claude to complete
    let status = child.wait()?;

    // Collect captured output from threads
    let stdout = stdout_handle.join().unwrap_or_default();
    let stderr = stderr_handle.join().unwrap_or_default();

    Ok(IterationResult {
        success: status.success(),
        exit_code: status.code(),
        stdout,
        stderr,
    })
}

/// Stream data from a pipe to an output writer while capturing it.
///
/// Reads lines from the pipe, writes them to the output immediately,
/// and returns the accumulated content.
#[allow(dead_code)] // Used by spawn_claude
fn stream_and_capture<R, W>(pipe: Option<R>, mut output: W) -> String
where
    R: std::io::Read + Send,
    W: Write,
{
    let Some(pipe) = pipe else {
        return String::new();
    };

    let reader = BufReader::new(pipe);
    let mut captured = String::new();

    for line in reader.lines() {
        match line {
            Ok(line) => {
                // Echo to output immediately for real-time streaming
                let _ = writeln!(output, "{}", line);
                let _ = output.flush();

                // Capture for later inspection
                captured.push_str(&line);
                captured.push('\n');
            }
            Err(_) => break,
        }
    }

    captured
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
            stdout: "output".to_string(),
            stderr: String::new(),
        };
        // Verify Debug trait is implemented
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("success: true"));
        assert!(debug_str.contains("exit_code: Some(0)"));
        assert!(debug_str.contains("stdout"));
    }

    #[test]
    fn test_stream_and_capture_with_data() {
        use std::io::Cursor;

        let input = "line1\nline2\nline3\n";
        let pipe = Some(Cursor::new(input.as_bytes().to_vec()));
        let mut output_buffer = Vec::new();

        let captured = stream_and_capture(pipe, &mut output_buffer);

        // Verify content was captured
        assert!(captured.contains("line1"));
        assert!(captured.contains("line2"));
        assert!(captured.contains("line3"));

        // Verify content was written to output
        let output_str = String::from_utf8_lossy(&output_buffer);
        assert!(output_str.contains("line1"));
        assert!(output_str.contains("line2"));
        assert!(output_str.contains("line3"));
    }

    #[test]
    fn test_stream_and_capture_empty_pipe() {
        let captured = stream_and_capture::<std::io::Empty, Vec<u8>>(None, Vec::new());
        assert_eq!(captured, "");
    }

    #[test]
    fn test_stream_and_capture_realtime_output() {
        // Test that streaming with cat subprocess works correctly
        let mut child = Command::new("cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn cat");

        let test_input = "Hello\nWorld\n";

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(test_input.as_bytes()).unwrap();
        }

        let stdout_pipe = child.stdout.take();
        let stderr_pipe = child.stderr.take();

        // Capture to buffers instead of real stdout/stderr for testing
        let mut stdout_buffer = Vec::new();
        let mut stderr_buffer = Vec::new();

        let stdout_captured = stream_and_capture(stdout_pipe, &mut stdout_buffer);
        let stderr_captured = stream_and_capture(stderr_pipe, &mut stderr_buffer);

        let status = child.wait().expect("Failed to wait on child");
        assert!(status.success());

        // Verify stdout was captured correctly
        assert!(stdout_captured.contains("Hello"));
        assert!(stdout_captured.contains("World"));

        // Verify it was also written to the output buffer
        let output_str = String::from_utf8_lossy(&stdout_buffer);
        assert!(output_str.contains("Hello"));
        assert!(output_str.contains("World"));

        // Stderr should be empty since cat doesn't produce stderr
        assert!(stderr_captured.is_empty());
    }
}
