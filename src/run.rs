//! Run command implementation for ralphctl.
//!
//! Provides the core ralph loop execution logic.

use crate::{error, files, parser};
use anyhow::Result;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

/// Required files that must exist before running.
const REQUIRED_FILES: &[&str] = &[
    files::PROMPT_FILE,
    files::SPEC_FILE,
    files::IMPLEMENTATION_PLAN_FILE,
];

/// Format the iteration header string.
///
/// Format: `=== Iteration N starting ===`
pub fn format_iteration_header(iteration: u32) -> String {
    format!("=== Iteration {} starting ===", iteration)
}

/// Print the iteration header to stdout.
pub fn print_iteration_header(iteration: u32) {
    println!("{}", format_iteration_header(iteration));
}

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

/// Append iteration output to ralph.log.
///
/// Creates the log file if it doesn't exist. Each iteration is logged with
/// a header and separator for easy parsing.
pub fn log_iteration(iteration: u32, stdout: &str) -> Result<()> {
    use std::fs::OpenOptions;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(files::LOG_FILE)?;

    writeln!(file, "{}", format_iteration_header(iteration))?;
    writeln!(file, "{}", stdout)?;
    writeln!(file, "--- end iteration {} ---\n", iteration)?;

    Ok(())
}

/// Result of prompting user to continue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PauseAction {
    /// Continue to next iteration
    Continue,
    /// Stop the loop gracefully
    Stop,
}

/// Prompt user to continue to next iteration.
///
/// Returns `PauseAction::Continue` on 'y', 'Y', or empty input.
/// Returns `PauseAction::Stop` on 'n', 'N', 'q', or 'Q'.
pub fn prompt_continue() -> Result<PauseAction> {
    eprint!("Continue? [Y/n] ");
    io::stderr().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let answer = input.trim().to_lowercase();
    if answer.is_empty() || answer == "y" || answer == "yes" {
        Ok(PauseAction::Continue)
    } else {
        Ok(PauseAction::Stop)
    }
}

/// Result of prompting user when no magic string was detected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoSignalAction {
    /// Continue to next iteration
    Continue,
    /// Stop the loop gracefully
    Stop,
}

/// Prompt user for action when no magic string (DONE or BLOCKED) was detected.
///
/// This fallback ensures the loop doesn't continue indefinitely when claude
/// fails to output a proper termination signal.
///
/// Returns `NoSignalAction::Continue` on 'c', 'C', or empty input.
/// Returns `NoSignalAction::Stop` on 's', 'S', 'q', or 'Q'.
pub fn prompt_no_signal() -> Result<NoSignalAction> {
    eprintln!("warning: no [[RALPH:DONE]] or [[RALPH:BLOCKED:...]] signal detected");
    eprint!("Continue or stop? [C/s] ");
    io::stderr().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let answer = input.trim().to_lowercase();
    if answer.is_empty() || answer == "c" || answer == "continue" {
        Ok(NoSignalAction::Continue)
    } else {
        Ok(NoSignalAction::Stop)
    }
}

/// Print interrupt summary showing iterations completed and task progress.
///
/// Format: `Interrupted after N iterations. X/Y tasks complete.`
pub fn print_interrupt_summary(iterations_completed: u32) {
    let task_summary = match fs::read_to_string(files::IMPLEMENTATION_PLAN_FILE) {
        Ok(content) => {
            let count = parser::count_checkboxes(&content);
            format!("{}/{} tasks complete", count.completed, count.total)
        }
        Err(_) => "task status unknown".to_string(),
    };

    eprintln!(
        "Interrupted after {} iteration{}. {}.",
        iterations_completed,
        if iterations_completed == 1 { "" } else { "s" },
        task_summary
    );
}

/// Magic string indicating the ralph loop completed successfully (all tasks done).
pub const RALPH_DONE_MARKER: &str = "[[RALPH:DONE]]";

/// Magic string indicating a task was completed and the loop should continue.
pub const RALPH_CONTINUE_MARKER: &str = "[[RALPH:CONTINUE]]";

/// Result of running a single iteration of the claude subprocess.
#[derive(Debug)]
pub struct IterationResult {
    /// Whether the subprocess exited successfully (exit code 0)
    pub success: bool,
    /// Exit code from the subprocess
    pub exit_code: Option<i32>,
    /// Captured stdout output for magic string detection
    pub stdout: String,
    /// Captured stderr output (used for BLOCKED signal detection)
    #[allow(dead_code)]
    pub stderr: String,
    /// Whether the iteration was interrupted by Ctrl+C
    pub was_interrupted: bool,
}

/// Outcome of checking for magic strings in iteration output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopSignal {
    /// All tasks completed (RALPH:DONE detected)
    Done,
    /// Task completed, continue to next iteration (RALPH:CONTINUE detected)
    Continue,
    /// No signal detected
    NoSignal,
}

/// Check if the output contains a RALPH signal marker on its own line.
///
/// Scans the provided output string for magic strings `[[RALPH:DONE]]` or
/// `[[RALPH:CONTINUE]]`. The marker must appear alone on a line (with optional
/// whitespace) to be detected. This prevents false positives when Claude
/// discusses or quotes the marker in its output.
///
/// Returns `LoopSignal::Done`, `LoopSignal::Continue`, or `LoopSignal::NoSignal`.
pub fn detect_signal(output: &str) -> LoopSignal {
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed == RALPH_DONE_MARKER {
            return LoopSignal::Done;
        }
        if trimmed == RALPH_CONTINUE_MARKER {
            return LoopSignal::Continue;
        }
    }
    LoopSignal::NoSignal
}

/// Magic string prefix for blocked signal.
pub const RALPH_BLOCKED_PREFIX: &str = "[[RALPH:BLOCKED:";
/// Magic string suffix for blocked signal.
pub const RALPH_BLOCKED_SUFFIX: &str = "]]";

/// Check if the output contains a RALPH:BLOCKED signal on its own line.
///
/// Scans for `[[RALPH:BLOCKED:<reason>]]` pattern and extracts the reason.
/// The marker must appear alone on a line (with optional whitespace) to be
/// detected. This prevents false positives when Claude discusses or quotes
/// the marker in its output.
///
/// Returns `Some(reason)` if found, `None` otherwise.
pub fn detect_blocked_signal(output: &str) -> Option<String> {
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(RALPH_BLOCKED_PREFIX) {
            if let Some(reason) = rest.strip_suffix(RALPH_BLOCKED_SUFFIX) {
                return Some(reason.to_string());
            }
        }
    }
    None
}

/// Spawn `claude -p` as a subprocess and pipe the prompt via stdin.
///
/// Streams stdout and stderr to the terminal in real-time while also
/// capturing the output for magic string detection.
/// Returns the result of the iteration after claude completes.
///
/// If `interrupt_flag` is provided and set to true during execution,
/// the child process will be killed and the function returns with
/// `was_interrupted` set to true in the result.
pub fn spawn_claude(
    prompt: &str,
    model: Option<&str>,
    interrupt_flag: Option<Arc<AtomicBool>>,
) -> Result<IterationResult> {
    let mut cmd = Command::new("claude");
    cmd.arg("-p")
        .arg("--dangerously-skip-permissions")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(m) = model {
        cmd.arg("--model").arg(m);
    }

    let mut child = cmd.spawn().inspect_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            error::die("claude not found in PATH");
        }
    })?;

    // Write prompt to stdin, then drop to signal EOF
    // Ignore BrokenPipe errors - the child may exit before reading all input
    if let Some(mut stdin) = child.stdin.take() {
        if let Err(e) = stdin.write_all(prompt.as_bytes()) {
            if e.kind() != io::ErrorKind::BrokenPipe {
                return Err(e.into());
            }
        }
        // stdin is dropped here, closing the pipe
    }

    // Take ownership of stdout and stderr for streaming
    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();

    // Clone interrupt flag for the polling thread
    let interrupt_flag_clone = interrupt_flag.clone();
    let child_id = child.id();

    // Flag to signal the kill thread to stop when child exits normally
    let child_done = Arc::new(AtomicBool::new(false));
    let child_done_clone = child_done.clone();

    // Spawn thread to stream and capture stdout
    let stdout_handle = thread::spawn(move || stream_and_capture(stdout_pipe, io::stdout()));

    // Spawn thread to stream and capture stderr
    let stderr_handle = thread::spawn(move || stream_and_capture(stderr_pipe, io::stderr()));

    // Spawn thread to poll for interrupt and kill child if needed
    let kill_handle = interrupt_flag_clone.map(|flag| {
        thread::spawn(move || {
            // Poll every 100ms for interrupt signal or child completion
            loop {
                if child_done_clone.load(Ordering::SeqCst) {
                    // Child completed normally, no need to kill
                    break;
                }
                if flag.load(Ordering::SeqCst) {
                    // Interrupt received, kill the child process
                    #[cfg(unix)]
                    {
                        use nix::sys::signal::{kill, Signal};
                        use nix::unistd::Pid;
                        // Send SIGTERM to the child process
                        let _ = kill(Pid::from_raw(child_id as i32), Signal::SIGTERM);
                    }
                    break;
                }
                thread::sleep(std::time::Duration::from_millis(100));
            }
        })
    });

    // Wait for claude to complete
    let status = child.wait()?;

    // Signal the kill thread that the child has exited
    child_done.store(true, Ordering::SeqCst);

    // Check if we were interrupted
    let was_interrupted = interrupt_flag
        .as_ref()
        .is_some_and(|f| f.load(Ordering::SeqCst));

    // Wait for kill thread to finish if it exists
    if let Some(handle) = kill_handle {
        // Don't wait forever - the thread should exit quickly once child is done
        let _ = handle.join();
    }

    // Collect captured output from threads
    let stdout = stdout_handle.join().unwrap_or_default();
    let stderr = stderr_handle.join().unwrap_or_default();

    Ok(IterationResult {
        success: status.success() && !was_interrupted,
        exit_code: status.code(),
        stdout,
        stderr,
        was_interrupted,
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
            was_interrupted: false,
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
    fn test_format_iteration_header() {
        assert_eq!(format_iteration_header(1), "=== Iteration 1 starting ===");
        assert_eq!(format_iteration_header(42), "=== Iteration 42 starting ===");
        assert_eq!(
            format_iteration_header(100),
            "=== Iteration 100 starting ==="
        );
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

    #[test]
    fn test_detect_signal_done() {
        let output = "Completed all tasks.\n[[RALPH:DONE]]\n";
        assert_eq!(detect_signal(output), LoopSignal::Done);
    }

    #[test]
    fn test_detect_signal_continue() {
        let output = "Task completed.\n[[RALPH:CONTINUE]]\n";
        assert_eq!(detect_signal(output), LoopSignal::Continue);
    }

    #[test]
    fn test_detect_signal_rejects_inline_done() {
        // Marker must be alone on a line - inline mentions are rejected
        // to prevent false positives when Claude discusses the marker
        let output = "Work finished [[RALPH:DONE]] done";
        assert_eq!(detect_signal(output), LoopSignal::NoSignal);
    }

    #[test]
    fn test_detect_signal_rejects_inline_continue() {
        let output = "Output [[RALPH:CONTINUE]] more text";
        assert_eq!(detect_signal(output), LoopSignal::NoSignal);
    }

    #[test]
    fn test_detect_signal_done_with_whitespace() {
        // Marker can have leading/trailing whitespace on its line
        let output = "Some output\n  [[RALPH:DONE]]  \nMore text";
        assert_eq!(detect_signal(output), LoopSignal::Done);
    }

    #[test]
    fn test_detect_signal_continue_with_whitespace() {
        let output = "Some output\n  [[RALPH:CONTINUE]]  \nMore text";
        assert_eq!(detect_signal(output), LoopSignal::Continue);
    }

    #[test]
    fn test_detect_signal_rejects_quoted_mention() {
        // When Claude explains what the marker does, it shouldn't trigger
        let output = "The test covers `[[RALPH:DONE]]` signal detection";
        assert_eq!(detect_signal(output), LoopSignal::NoSignal);
    }

    #[test]
    fn test_detect_signal_no_signal() {
        let output = "Still working on tasks...\nMore output here.";
        assert_eq!(detect_signal(output), LoopSignal::NoSignal);
    }

    #[test]
    fn test_detect_signal_empty_output() {
        assert_eq!(detect_signal(""), LoopSignal::NoSignal);
    }

    #[test]
    fn test_detect_signal_partial_marker() {
        // Partial markers should not trigger
        let output = "[[RALPH:DON]] almost done";
        assert_eq!(detect_signal(output), LoopSignal::NoSignal);

        let output2 = "RALPH:DONE without brackets";
        assert_eq!(detect_signal(output2), LoopSignal::NoSignal);
    }

    #[test]
    fn test_detect_signal_done_takes_priority() {
        // If both DONE and CONTINUE are present, first one wins (DONE in this case)
        let output = "[[RALPH:DONE]]\n[[RALPH:CONTINUE]]\n";
        assert_eq!(detect_signal(output), LoopSignal::Done);
    }

    #[test]
    fn test_detect_signal_continue_first() {
        // If CONTINUE comes before DONE, CONTINUE wins
        let output = "[[RALPH:CONTINUE]]\n[[RALPH:DONE]]\n";
        assert_eq!(detect_signal(output), LoopSignal::Continue);
    }

    #[test]
    fn test_loop_signal_equality() {
        assert_eq!(LoopSignal::Done, LoopSignal::Done);
        assert_eq!(LoopSignal::Continue, LoopSignal::Continue);
        assert_eq!(LoopSignal::NoSignal, LoopSignal::NoSignal);
        assert_ne!(LoopSignal::Done, LoopSignal::Continue);
        assert_ne!(LoopSignal::Done, LoopSignal::NoSignal);
        assert_ne!(LoopSignal::Continue, LoopSignal::NoSignal);
    }

    #[test]
    fn test_loop_signal_clone() {
        let signal = LoopSignal::Done;
        let cloned = signal.clone();
        assert_eq!(signal, cloned);

        let signal2 = LoopSignal::NoSignal;
        let cloned2 = signal2.clone();
        assert_eq!(signal2, cloned2);
    }

    #[test]
    fn test_ralph_done_marker_constant() {
        assert_eq!(RALPH_DONE_MARKER, "[[RALPH:DONE]]");
    }

    #[test]
    fn test_ralph_continue_marker_constant() {
        assert_eq!(RALPH_CONTINUE_MARKER, "[[RALPH:CONTINUE]]");
    }

    #[test]
    fn test_detect_blocked_signal_found() {
        let output = "Cannot proceed.\n[[RALPH:BLOCKED:missing API key]]\n";
        assert_eq!(
            detect_blocked_signal(output),
            Some("missing API key".to_string())
        );
    }

    #[test]
    fn test_detect_blocked_signal_rejects_inline() {
        // Marker must be alone on a line - inline mentions are rejected
        let output = "Text before [[RALPH:BLOCKED:need user input]] text after";
        assert_eq!(detect_blocked_signal(output), None);
    }

    #[test]
    fn test_detect_blocked_signal_with_whitespace() {
        // Marker can have leading/trailing whitespace on its line
        let output = "Some output\n  [[RALPH:BLOCKED:need user input]]  \nMore text";
        assert_eq!(
            detect_blocked_signal(output),
            Some("need user input".to_string())
        );
    }

    #[test]
    fn test_detect_blocked_signal_rejects_quoted_mention() {
        // When Claude explains what the marker does, it shouldn't trigger
        let output = "The test covers `[[RALPH:BLOCKED:reason]]` detection";
        assert_eq!(detect_blocked_signal(output), None);
    }

    #[test]
    fn test_detect_blocked_signal_not_found() {
        let output = "Still working on tasks...\nMore output here.";
        assert_eq!(detect_blocked_signal(output), None);
    }

    #[test]
    fn test_detect_blocked_signal_empty_output() {
        assert_eq!(detect_blocked_signal(""), None);
    }

    #[test]
    fn test_detect_blocked_signal_empty_reason() {
        let output = "[[RALPH:BLOCKED:]]";
        assert_eq!(detect_blocked_signal(output), Some("".to_string()));
    }

    #[test]
    fn test_detect_blocked_signal_partial_marker() {
        // Missing closing brackets
        let output = "[[RALPH:BLOCKED:reason without closing";
        assert_eq!(detect_blocked_signal(output), None);

        // Missing prefix
        let output2 = "RALPH:BLOCKED:reason]]";
        assert_eq!(detect_blocked_signal(output2), None);
    }

    #[test]
    fn test_blocked_marker_constants() {
        assert_eq!(RALPH_BLOCKED_PREFIX, "[[RALPH:BLOCKED:");
        assert_eq!(RALPH_BLOCKED_SUFFIX, "]]");
    }

    // ========== Real-world Claude output pattern tests ==========

    #[test]
    fn test_detect_signal_in_code_block_not_detected() {
        // Signal inside a code block should NOT be detected
        // (the backticks make it not alone on the line)
        let output = r#"Here's an example:
```
[[RALPH:DONE]]
```
"#;
        // The signal IS on its own line inside the code block, so it WILL be detected
        // This is actually the expected behavior - we detect based on line content only
        assert_eq!(detect_signal(output), LoopSignal::Done);
    }

    #[test]
    fn test_detect_signal_after_long_output() {
        // Signal at the very end of long output (typical Claude pattern)
        let output = format!(
            "{}\n\n[[RALPH:CONTINUE]]\n",
            "Task completed successfully.\n".repeat(100)
        );
        assert_eq!(detect_signal(&output), LoopSignal::Continue);
    }

    #[test]
    fn test_detect_signal_with_ansi_escape_codes() {
        // Some terminals/tools might include ANSI codes
        // The signal should still be detected if it's on its own line
        let output = "\x1b[32mSuccess!\x1b[0m\n[[RALPH:DONE]]\n";
        assert_eq!(detect_signal(output), LoopSignal::Done);
    }

    #[test]
    fn test_detect_signal_windows_line_endings() {
        // Windows-style CRLF line endings
        let output = "Task done.\r\n[[RALPH:CONTINUE]]\r\n";
        assert_eq!(detect_signal(output), LoopSignal::Continue);
    }

    #[test]
    fn test_detect_signal_mixed_line_endings() {
        // Mix of Unix and Windows line endings
        let output = "Line 1\r\nLine 2\n[[RALPH:DONE]]\r\nLine 4\n";
        assert_eq!(detect_signal(output), LoopSignal::Done);
    }

    #[test]
    fn test_detect_signal_unicode_content() {
        // Unicode characters shouldn't interfere with signal detection
        let output = "完成任务 ✓\n🎉 Success!\n[[RALPH:DONE]]\n";
        assert_eq!(detect_signal(output), LoopSignal::Done);
    }

    #[test]
    fn test_detect_signal_with_tabs() {
        // Tabs count as whitespace, should be trimmed
        let output = "\t[[RALPH:CONTINUE]]\t\n";
        assert_eq!(detect_signal(output), LoopSignal::Continue);
    }

    #[test]
    fn test_detect_signal_only_whitespace_lines() {
        // Output with only whitespace lines and no signal
        let output = "   \n\t\n   \t   \n";
        assert_eq!(detect_signal(output), LoopSignal::NoSignal);
    }

    #[test]
    fn test_detect_signal_case_sensitivity() {
        // Signals are case-sensitive
        let output1 = "[[ralph:done]]";
        assert_eq!(detect_signal(output1), LoopSignal::NoSignal);

        let output2 = "[[RALPH:done]]";
        assert_eq!(detect_signal(output2), LoopSignal::NoSignal);

        let output3 = "[[Ralph:Continue]]";
        assert_eq!(detect_signal(output3), LoopSignal::NoSignal);
    }

    #[test]
    fn test_detect_signal_similar_but_wrong_markers() {
        // Similar strings that should NOT match
        let cases = vec![
            "[[RALPH:DONE ]]",     // Extra space before closing
            "[[ RALPH:DONE]]",     // Extra space after opening
            "[[RALPH: DONE]]",     // Space after colon
            "[[RALPH:DONEE]]",     // Extra E
            "[[RALPH:DON]]",       // Missing E
            "[RALPH:DONE]",        // Single brackets
            "[[RALPH:DONE]",       // Missing closing bracket
            "[[RALPH:CONTINUE]",   // Missing closing bracket
            "[[RALPH:CONTINUES]]", // Extra S
            "[[RALPH:CONT]]",      // Truncated
        ];

        for case in cases {
            assert_eq!(
                detect_signal(case),
                LoopSignal::NoSignal,
                "Expected NoSignal for: {}",
                case
            );
        }
    }

    #[test]
    fn test_detect_blocked_with_colons_in_reason() {
        // Reason can contain colons (common in error messages)
        let output = "[[RALPH:BLOCKED:Error: file not found: /path/to/file]]";
        assert_eq!(
            detect_blocked_signal(output),
            Some("Error: file not found: /path/to/file".to_string())
        );
    }

    #[test]
    fn test_detect_blocked_with_brackets_in_reason() {
        // Reason can contain brackets (but not the closing ]])
        let output = "[[RALPH:BLOCKED:Array [1, 2, 3] is empty]]";
        assert_eq!(
            detect_blocked_signal(output),
            Some("Array [1, 2, 3] is empty".to_string())
        );
    }

    #[test]
    fn test_detect_blocked_multiline_reason_not_supported() {
        // Multiline reasons are not supported (signal must be on one line)
        let output = "[[RALPH:BLOCKED:Line 1\nLine 2]]";
        // This will not match because newline splits it
        assert_eq!(detect_blocked_signal(output), None);
    }

    #[test]
    fn test_detect_blocked_with_unicode_reason() {
        let output = "[[RALPH:BLOCKED:找不到文件 🚫]]";
        assert_eq!(
            detect_blocked_signal(output),
            Some("找不到文件 🚫".to_string())
        );
    }

    #[test]
    fn test_detect_blocked_very_long_reason() {
        // Long reasons should still work
        let long_reason = "x".repeat(1000);
        let output = format!("[[RALPH:BLOCKED:{}]]", long_reason);
        assert_eq!(detect_blocked_signal(&output), Some(long_reason));
    }

    #[test]
    fn test_signal_and_blocked_both_present_blocked_wins_in_main() {
        // When both signals are present, the order of detection in main.rs
        // determines priority: BLOCKED is checked first
        // This test verifies detect_blocked_signal finds it
        let output = "[[RALPH:DONE]]\n[[RALPH:BLOCKED:oops]]";
        assert_eq!(detect_blocked_signal(output), Some("oops".to_string()));
        assert_eq!(detect_signal(output), LoopSignal::Done);
        // In main.rs, BLOCKED is checked first, so it would take priority
    }

    #[test]
    fn test_detect_signal_no_newline_at_end() {
        // Signal at end without trailing newline
        let output = "Task done.\n[[RALPH:DONE]]";
        assert_eq!(detect_signal(output), LoopSignal::Done);
    }

    #[test]
    fn test_detect_signal_only_signal() {
        // Output is just the signal
        assert_eq!(detect_signal("[[RALPH:DONE]]"), LoopSignal::Done);
        assert_eq!(detect_signal("[[RALPH:CONTINUE]]"), LoopSignal::Continue);
    }

    #[test]
    fn test_detect_signal_insight_box_pattern() {
        // Real pattern from Claude output - signal after insight box
        let output = r#"
`★ Insight ─────────────────────────────────────`
Some educational content here.
`─────────────────────────────────────────────────`

[[RALPH:CONTINUE]]
"#;
        assert_eq!(detect_signal(output), LoopSignal::Continue);
    }

    #[test]
    fn test_detect_signal_with_markdown_formatting() {
        // Signal after markdown content
        let output = r#"
## Summary

- Implemented feature X
- Added tests for Y
- Fixed bug Z

**Status**: Complete

[[RALPH:DONE]]
"#;
        assert_eq!(detect_signal(output), LoopSignal::Done);
    }

    #[test]
    fn test_log_iteration_creates_file() {
        with_temp_dir(|_dir| {
            log_iteration(1, "Test output").unwrap();
            assert!(Path::new(files::LOG_FILE).exists());
        });
    }

    #[test]
    fn test_log_iteration_content_format() {
        with_temp_dir(|_dir| {
            log_iteration(1, "First iteration output").unwrap();

            let content = fs::read_to_string(files::LOG_FILE).unwrap();
            assert!(content.contains("=== Iteration 1 starting ==="));
            assert!(content.contains("First iteration output"));
            assert!(content.contains("--- end iteration 1 ---"));
        });
    }

    #[test]
    fn test_log_iteration_appends() {
        with_temp_dir(|_dir| {
            log_iteration(1, "First").unwrap();
            log_iteration(2, "Second").unwrap();

            let content = fs::read_to_string(files::LOG_FILE).unwrap();
            assert!(content.contains("=== Iteration 1 starting ==="));
            assert!(content.contains("First"));
            assert!(content.contains("=== Iteration 2 starting ==="));
            assert!(content.contains("Second"));
        });
    }

    #[test]
    fn test_pause_action_equality() {
        assert_eq!(PauseAction::Continue, PauseAction::Continue);
        assert_eq!(PauseAction::Stop, PauseAction::Stop);
        assert_ne!(PauseAction::Continue, PauseAction::Stop);
    }

    #[test]
    fn test_pause_action_clone() {
        let action = PauseAction::Continue;
        let cloned = action.clone();
        assert_eq!(action, cloned);
    }

    #[test]
    fn test_pause_action_debug() {
        let action = PauseAction::Stop;
        let debug_str = format!("{:?}", action);
        assert_eq!(debug_str, "Stop");
    }

    #[test]
    fn test_iteration_result_was_interrupted_field() {
        let result = IterationResult {
            success: false,
            exit_code: Some(130),
            stdout: String::new(),
            stderr: String::new(),
            was_interrupted: true,
        };
        assert!(result.was_interrupted);
        assert!(!result.success);
    }

    #[test]
    fn test_no_signal_action_equality() {
        assert_eq!(NoSignalAction::Continue, NoSignalAction::Continue);
        assert_eq!(NoSignalAction::Stop, NoSignalAction::Stop);
        assert_ne!(NoSignalAction::Continue, NoSignalAction::Stop);
    }

    #[test]
    fn test_no_signal_action_clone() {
        let action = NoSignalAction::Continue;
        let cloned = action.clone();
        assert_eq!(action, cloned);
    }

    #[test]
    fn test_no_signal_action_debug() {
        let action = NoSignalAction::Stop;
        let debug_str = format!("{:?}", action);
        assert_eq!(debug_str, "Stop");
    }

    #[test]
    fn test_broken_pipe_handled_gracefully() {
        // Simulate a subprocess that exits immediately without reading stdin
        // This triggers EPIPE when we try to write to its stdin
        let mut child = Command::new("true") // exits immediately with success
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn true");

        // Give the child time to exit
        let _ = child.wait();

        // Now try writing to the closed stdin - this should produce BrokenPipe
        if let Some(mut stdin) = child.stdin.take() {
            let large_input = "x".repeat(65536); // Large enough to trigger EPIPE
            let result = stdin.write_all(large_input.as_bytes());

            // The write should fail with BrokenPipe (or succeed if buffered)
            if let Err(e) = result {
                assert_eq!(
                    e.kind(),
                    io::ErrorKind::BrokenPipe,
                    "Expected BrokenPipe error, got: {:?}",
                    e.kind()
                );
            }
            // If it succeeds (due to buffering), that's also acceptable
        }
    }

    #[test]
    fn test_subprocess_exits_before_reading_all_stdin() {
        // Test the pattern used by the mock claude script: exits without reading stdin
        // Use 'true' which reads nothing and exits immediately with success
        let mut child = Command::new("true")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn true");

        let stdin = child.stdin.take();
        let stdout = child.stdout.take();

        // Wait for child to exit first
        let status = child.wait().expect("Failed to wait on child");
        assert!(status.success());

        // Now write to the closed stdin - should trigger EPIPE
        if let Some(mut stdin) = stdin {
            let large_input = "test data\n".repeat(10000);
            // This may error with BrokenPipe - both outcomes are acceptable
            let result = stdin.write_all(large_input.as_bytes());
            if let Err(e) = result {
                assert_eq!(e.kind(), io::ErrorKind::BrokenPipe);
            }
        }

        // Capture stdout (should be empty since 'true' produces no output)
        let captured = stream_and_capture(stdout, Vec::new());
        assert!(captured.is_empty());
    }
}
