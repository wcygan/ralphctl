//! Reverse mode implementation for ralphctl.
//!
//! Provides investigation loop logic distinct from forward mode.
//! Reverse mode is used for autonomous investigation of codebases
//! to answer questions—diagnosing bugs, understanding systems, or
//! mapping dependencies before changes.

#![allow(dead_code)] // Components used by future reverse mode implementation

use crate::files::QUESTION_FILE;
use crate::run;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Reverse mode signal types.
///
/// These signals control the reverse mode investigation loop.
/// Detection priority: BLOCKED → FOUND → INCONCLUSIVE → CONTINUE
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReverseSignal {
    /// Still investigating, more hypotheses to explore
    Continue,
    /// Question answered, FINDINGS.md written
    Found(String),
    /// Cannot determine answer, FINDINGS.md written with what was tried
    Inconclusive(String),
    /// Cannot proceed, requires human intervention
    Blocked(String),
    /// No signal detected in output
    NoSignal,
}

/// Magic string prefix for FOUND signal.
pub const RALPH_FOUND_PREFIX: &str = "[[RALPH:FOUND:";

/// Magic string prefix for INCONCLUSIVE signal.
pub const RALPH_INCONCLUSIVE_PREFIX: &str = "[[RALPH:INCONCLUSIVE:";

/// Magic string suffix (shared with other signals).
const SIGNAL_SUFFIX: &str = "]]";

/// Minimal template for QUESTION.md when created without an argument.
const QUESTION_TEMPLATE: &str = r#"# Investigation Question

Describe what you want to investigate...
"#;

/// Read the investigation question from QUESTION.md.
///
/// Returns the full contents of the QUESTION.md file.
///
/// # Errors
///
/// Returns an error if QUESTION.md does not exist or cannot be read.
pub fn read_question(dir: &Path) -> Result<String> {
    let path = dir.join(QUESTION_FILE);
    fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))
}

/// Create a minimal QUESTION.md template.
///
/// Writes a placeholder template for the user to fill in.
///
/// # Errors
///
/// Returns an error if the file cannot be written.
pub fn create_question_template(dir: &Path) -> Result<()> {
    let path = dir.join(QUESTION_FILE);
    fs::write(&path, QUESTION_TEMPLATE)
        .with_context(|| format!("failed to write {}", path.display()))
}

/// Write an investigation question to QUESTION.md.
///
/// Creates QUESTION.md with the provided question formatted
/// with the standard header and optional context section.
///
/// # Errors
///
/// Returns an error if the file cannot be written.
pub fn write_question(dir: &Path, question: &str) -> Result<()> {
    let path = dir.join(QUESTION_FILE);
    let content = format!(
        r#"# Investigation Question

{}

## Context (Optional)

<Add any additional context here>
"#,
        question
    );
    fs::write(&path, content).with_context(|| format!("failed to write {}", path.display()))
}

/// Detect reverse mode signals in output.
///
/// Scans the provided output string for reverse mode magic strings.
/// Each marker must appear alone on a line (with optional whitespace)
/// to be detected. This prevents false positives when Claude discusses
/// or quotes the markers in its output.
///
/// Detection priority: BLOCKED → FOUND → INCONCLUSIVE → CONTINUE
///
/// This priority ensures that:
/// - Blockers are always surfaced first (they require human intervention)
/// - FOUND takes precedence over INCONCLUSIVE (success over failure)
/// - Both take precedence over CONTINUE (terminal over continuation)
pub fn detect_reverse_signal(output: &str) -> ReverseSignal {
    // Priority 1: Check for BLOCKED signal (requires human intervention)
    if let Some(reason) = run::detect_blocked_signal(output) {
        return ReverseSignal::Blocked(reason);
    }

    // Priority 2: Check for FOUND signal (question answered)
    if let Some(summary) = detect_found_signal(output) {
        return ReverseSignal::Found(summary);
    }

    // Priority 3: Check for INCONCLUSIVE signal (cannot determine answer)
    if let Some(reason) = detect_inconclusive_signal(output) {
        return ReverseSignal::Inconclusive(reason);
    }

    // Priority 4: Check for CONTINUE signal (still investigating)
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed == run::RALPH_CONTINUE_MARKER {
            return ReverseSignal::Continue;
        }
    }

    ReverseSignal::NoSignal
}

/// Check if the output contains a RALPH:FOUND signal on its own line.
///
/// Scans for `[[RALPH:FOUND:<summary>]]` pattern and extracts the summary.
/// The marker must appear alone on a line (with optional whitespace).
///
/// Returns `Some(summary)` if found, `None` otherwise.
fn detect_found_signal(output: &str) -> Option<String> {
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(RALPH_FOUND_PREFIX) {
            if let Some(summary) = rest.strip_suffix(SIGNAL_SUFFIX) {
                return Some(summary.to_string());
            }
        }
    }
    None
}

/// Check if the output contains a RALPH:INCONCLUSIVE signal on its own line.
///
/// Scans for `[[RALPH:INCONCLUSIVE:<reason>]]` pattern and extracts the reason.
/// The marker must appear alone on a line (with optional whitespace).
///
/// Returns `Some(reason)` if found, `None` otherwise.
fn detect_inconclusive_signal(output: &str) -> Option<String> {
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(RALPH_INCONCLUSIVE_PREFIX) {
            if let Some(reason) = rest.strip_suffix(SIGNAL_SUFFIX) {
                return Some(reason.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reverse_signal_equality() {
        assert_eq!(ReverseSignal::Continue, ReverseSignal::Continue);
        assert_eq!(ReverseSignal::NoSignal, ReverseSignal::NoSignal);
        assert_eq!(
            ReverseSignal::Found("answer".to_string()),
            ReverseSignal::Found("answer".to_string())
        );
        assert_eq!(
            ReverseSignal::Inconclusive("reason".to_string()),
            ReverseSignal::Inconclusive("reason".to_string())
        );
        assert_eq!(
            ReverseSignal::Blocked("blocker".to_string()),
            ReverseSignal::Blocked("blocker".to_string())
        );
    }

    #[test]
    fn test_reverse_signal_inequality() {
        assert_ne!(ReverseSignal::Continue, ReverseSignal::NoSignal);
        assert_ne!(
            ReverseSignal::Found("a".to_string()),
            ReverseSignal::Found("b".to_string())
        );
        assert_ne!(
            ReverseSignal::Found("x".to_string()),
            ReverseSignal::Inconclusive("x".to_string())
        );
    }

    #[test]
    fn test_reverse_signal_clone() {
        let signal = ReverseSignal::Found("discovery".to_string());
        let cloned = signal.clone();
        assert_eq!(signal, cloned);

        let signal2 = ReverseSignal::Continue;
        let cloned2 = signal2.clone();
        assert_eq!(signal2, cloned2);
    }

    #[test]
    fn test_reverse_signal_debug() {
        let signal = ReverseSignal::Found("test".to_string());
        let debug_str = format!("{:?}", signal);
        assert!(debug_str.contains("Found"));
        assert!(debug_str.contains("test"));

        let signal2 = ReverseSignal::Continue;
        let debug_str2 = format!("{:?}", signal2);
        assert_eq!(debug_str2, "Continue");

        let signal3 = ReverseSignal::NoSignal;
        let debug_str3 = format!("{:?}", signal3);
        assert_eq!(debug_str3, "NoSignal");
    }

    #[test]
    fn test_reverse_signal_blocked_with_reason() {
        let reason = "missing credentials".to_string();
        let signal = ReverseSignal::Blocked(reason.clone());
        if let ReverseSignal::Blocked(r) = signal {
            assert_eq!(r, reason);
        } else {
            panic!("Expected Blocked variant");
        }
    }

    #[test]
    fn test_reverse_signal_inconclusive_with_reason() {
        let reason = "not enough evidence".to_string();
        let signal = ReverseSignal::Inconclusive(reason.clone());
        if let ReverseSignal::Inconclusive(r) = signal {
            assert_eq!(r, reason);
        } else {
            panic!("Expected Inconclusive variant");
        }
    }

    // ========== Signal marker constant tests ==========

    #[test]
    fn test_ralph_found_prefix_constant() {
        assert_eq!(RALPH_FOUND_PREFIX, "[[RALPH:FOUND:");
    }

    #[test]
    fn test_ralph_inconclusive_prefix_constant() {
        assert_eq!(RALPH_INCONCLUSIVE_PREFIX, "[[RALPH:INCONCLUSIVE:");
    }

    // ========== detect_reverse_signal() tests ==========

    #[test]
    fn test_detect_reverse_signal_continue() {
        let output = "Still investigating.\n[[RALPH:CONTINUE]]\n";
        assert_eq!(detect_reverse_signal(output), ReverseSignal::Continue);
    }

    #[test]
    fn test_detect_reverse_signal_found() {
        let output = "Question answered.\n[[RALPH:FOUND:The bug is in auth.rs:42]]\n";
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Found("The bug is in auth.rs:42".to_string())
        );
    }

    #[test]
    fn test_detect_reverse_signal_inconclusive() {
        let output = "Cannot determine.\n[[RALPH:INCONCLUSIVE:insufficient evidence]]\n";
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Inconclusive("insufficient evidence".to_string())
        );
    }

    #[test]
    fn test_detect_reverse_signal_blocked() {
        let output = "Cannot proceed.\n[[RALPH:BLOCKED:need database access]]\n";
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Blocked("need database access".to_string())
        );
    }

    #[test]
    fn test_detect_reverse_signal_no_signal() {
        let output = "Still working on the investigation...";
        assert_eq!(detect_reverse_signal(output), ReverseSignal::NoSignal);
    }

    #[test]
    fn test_detect_reverse_signal_empty_output() {
        assert_eq!(detect_reverse_signal(""), ReverseSignal::NoSignal);
    }

    // ========== Signal with whitespace tests ==========

    #[test]
    fn test_detect_reverse_signal_continue_with_whitespace() {
        let output = "Output\n  [[RALPH:CONTINUE]]  \nMore text";
        assert_eq!(detect_reverse_signal(output), ReverseSignal::Continue);
    }

    #[test]
    fn test_detect_reverse_signal_found_with_whitespace() {
        let output = "Output\n  [[RALPH:FOUND:answer]]  \nMore text";
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Found("answer".to_string())
        );
    }

    #[test]
    fn test_detect_reverse_signal_inconclusive_with_whitespace() {
        let output = "Output\n  [[RALPH:INCONCLUSIVE:reason]]  \nMore text";
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Inconclusive("reason".to_string())
        );
    }

    #[test]
    fn test_detect_reverse_signal_blocked_with_whitespace() {
        let output = "Output\n  [[RALPH:BLOCKED:reason]]  \nMore text";
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Blocked("reason".to_string())
        );
    }

    // ========== Signal rejection tests (inline mentions) ==========

    #[test]
    fn test_detect_reverse_signal_rejects_inline_continue() {
        let output = "Text [[RALPH:CONTINUE]] more text";
        assert_eq!(detect_reverse_signal(output), ReverseSignal::NoSignal);
    }

    #[test]
    fn test_detect_reverse_signal_rejects_inline_found() {
        let output = "Text [[RALPH:FOUND:answer]] more text";
        assert_eq!(detect_reverse_signal(output), ReverseSignal::NoSignal);
    }

    #[test]
    fn test_detect_reverse_signal_rejects_inline_inconclusive() {
        let output = "Text [[RALPH:INCONCLUSIVE:reason]] more text";
        assert_eq!(detect_reverse_signal(output), ReverseSignal::NoSignal);
    }

    #[test]
    fn test_detect_reverse_signal_rejects_inline_blocked() {
        let output = "Text [[RALPH:BLOCKED:reason]] more text";
        assert_eq!(detect_reverse_signal(output), ReverseSignal::NoSignal);
    }

    // ========== Signal priority tests ==========

    #[test]
    fn test_priority_blocked_over_found() {
        // BLOCKED takes priority over FOUND
        let output = "[[RALPH:FOUND:answer]]\n[[RALPH:BLOCKED:need help]]";
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Blocked("need help".to_string())
        );
    }

    #[test]
    fn test_priority_blocked_over_inconclusive() {
        // BLOCKED takes priority over INCONCLUSIVE
        let output = "[[RALPH:INCONCLUSIVE:unsure]]\n[[RALPH:BLOCKED:blocked]]";
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Blocked("blocked".to_string())
        );
    }

    #[test]
    fn test_priority_blocked_over_continue() {
        // BLOCKED takes priority over CONTINUE
        let output = "[[RALPH:CONTINUE]]\n[[RALPH:BLOCKED:stopped]]";
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Blocked("stopped".to_string())
        );
    }

    #[test]
    fn test_priority_found_over_inconclusive() {
        // FOUND takes priority over INCONCLUSIVE
        let output = "[[RALPH:INCONCLUSIVE:maybe]]\n[[RALPH:FOUND:definitely]]";
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Found("definitely".to_string())
        );
    }

    #[test]
    fn test_priority_found_over_continue() {
        // FOUND takes priority over CONTINUE
        let output = "[[RALPH:CONTINUE]]\n[[RALPH:FOUND:done]]";
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Found("done".to_string())
        );
    }

    #[test]
    fn test_priority_inconclusive_over_continue() {
        // INCONCLUSIVE takes priority over CONTINUE
        let output = "[[RALPH:CONTINUE]]\n[[RALPH:INCONCLUSIVE:giving up]]";
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Inconclusive("giving up".to_string())
        );
    }

    #[test]
    fn test_priority_all_signals_blocked_wins() {
        // When all four signals are present, BLOCKED wins
        let output =
            "[[RALPH:CONTINUE]]\n[[RALPH:FOUND:a]]\n[[RALPH:INCONCLUSIVE:b]]\n[[RALPH:BLOCKED:c]]";
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Blocked("c".to_string())
        );
    }

    #[test]
    fn test_priority_found_inconclusive_continue() {
        // When FOUND, INCONCLUSIVE, and CONTINUE are present, FOUND wins
        let output = "[[RALPH:CONTINUE]]\n[[RALPH:INCONCLUSIVE:x]]\n[[RALPH:FOUND:y]]";
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Found("y".to_string())
        );
    }

    // ========== Empty and special content tests ==========

    #[test]
    fn test_detect_found_empty_summary() {
        let output = "[[RALPH:FOUND:]]";
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Found("".to_string())
        );
    }

    #[test]
    fn test_detect_inconclusive_empty_reason() {
        let output = "[[RALPH:INCONCLUSIVE:]]";
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Inconclusive("".to_string())
        );
    }

    #[test]
    fn test_detect_found_with_colons() {
        // Summary can contain colons (common in file:line references)
        let output = "[[RALPH:FOUND:Error in src/main.rs:42:10]]";
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Found("Error in src/main.rs:42:10".to_string())
        );
    }

    #[test]
    fn test_detect_inconclusive_with_colons() {
        let output = "[[RALPH:INCONCLUSIVE:tried files: a.rs, b.rs, c.rs]]";
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Inconclusive("tried files: a.rs, b.rs, c.rs".to_string())
        );
    }

    #[test]
    fn test_detect_found_with_brackets() {
        // Summary can contain brackets (but not closing ]])
        let output = "[[RALPH:FOUND:Array [1, 2, 3] was empty]]";
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Found("Array [1, 2, 3] was empty".to_string())
        );
    }

    #[test]
    fn test_detect_found_with_unicode() {
        let output = "[[RALPH:FOUND:答案是 42 🎉]]";
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Found("答案是 42 🎉".to_string())
        );
    }

    #[test]
    fn test_detect_inconclusive_with_unicode() {
        let output = "[[RALPH:INCONCLUSIVE:找不到答案 😕]]";
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Inconclusive("找不到答案 😕".to_string())
        );
    }

    #[test]
    fn test_detect_found_very_long_summary() {
        let long_summary = "x".repeat(1000);
        let output = format!("[[RALPH:FOUND:{}]]", long_summary);
        assert_eq!(
            detect_reverse_signal(&output),
            ReverseSignal::Found(long_summary)
        );
    }

    // ========== Partial/malformed signal tests ==========

    #[test]
    fn test_detect_found_missing_closing_brackets() {
        let output = "[[RALPH:FOUND:answer";
        assert_eq!(detect_reverse_signal(output), ReverseSignal::NoSignal);
    }

    #[test]
    fn test_detect_inconclusive_missing_closing_brackets() {
        let output = "[[RALPH:INCONCLUSIVE:reason";
        assert_eq!(detect_reverse_signal(output), ReverseSignal::NoSignal);
    }

    #[test]
    fn test_detect_found_single_bracket() {
        let output = "[RALPH:FOUND:answer]";
        assert_eq!(detect_reverse_signal(output), ReverseSignal::NoSignal);
    }

    #[test]
    fn test_detect_case_sensitivity() {
        // Signals are case-sensitive
        let output1 = "[[ralph:found:answer]]";
        assert_eq!(detect_reverse_signal(output1), ReverseSignal::NoSignal);

        let output2 = "[[RALPH:found:answer]]";
        assert_eq!(detect_reverse_signal(output2), ReverseSignal::NoSignal);

        let output3 = "[[Ralph:Found:answer]]";
        assert_eq!(detect_reverse_signal(output3), ReverseSignal::NoSignal);
    }

    // ========== Real-world output pattern tests ==========

    #[test]
    fn test_detect_signal_after_investigation_output() {
        let output = r#"
## Hypothesis 3: Database connection pooling

Examined connection settings in config/database.yml.
Found connection pool size was set to 1.

- [x] Checked database config — pool_size=1
- **Result:** Confirmed

The root cause is the database connection pool being set to 1.

[[RALPH:FOUND:Root cause is pool_size=1 in config/database.yml]]
"#;
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Found("Root cause is pool_size=1 in config/database.yml".to_string())
        );
    }

    #[test]
    fn test_detect_signal_inconclusive_after_investigation() {
        let output = r#"
## Dead Ends

- Checked auth.rs - no issues found
- Checked middleware - working correctly
- Checked database - connections OK

After examining all components, I cannot determine the root cause.

[[RALPH:INCONCLUSIVE:Exhausted all hypotheses, no clear root cause found]]
"#;
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Inconclusive(
                "Exhausted all hypotheses, no clear root cause found".to_string()
            )
        );
    }

    #[test]
    fn test_detect_signal_continue_after_hypothesis() {
        let output = r#"
## Hypothesis 1: Race condition

- [x] Examined thread spawning — looks safe
- [ ] Check mutex usage
- **Result:** In Progress

More investigation needed.

[[RALPH:CONTINUE]]
"#;
        assert_eq!(detect_reverse_signal(output), ReverseSignal::Continue);
    }

    #[test]
    fn test_detect_signal_with_markdown_formatting() {
        let output = r#"
**Status**: Investigation complete

| File | Issue |
|------|-------|
| auth.rs | Missing null check |

[[RALPH:FOUND:Missing null check in auth.rs:157]]
"#;
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Found("Missing null check in auth.rs:157".to_string())
        );
    }

    #[test]
    fn test_detect_signal_windows_line_endings() {
        let output = "Found it.\r\n[[RALPH:FOUND:answer]]\r\n";
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Found("answer".to_string())
        );
    }

    #[test]
    fn test_detect_signal_no_trailing_newline() {
        let output = "Done.\n[[RALPH:FOUND:answer]]";
        assert_eq!(
            detect_reverse_signal(output),
            ReverseSignal::Found("answer".to_string())
        );
    }

    #[test]
    fn test_detect_signal_only_signal() {
        assert_eq!(
            detect_reverse_signal("[[RALPH:FOUND:x]]"),
            ReverseSignal::Found("x".to_string())
        );
        assert_eq!(
            detect_reverse_signal("[[RALPH:INCONCLUSIVE:y]]"),
            ReverseSignal::Inconclusive("y".to_string())
        );
        assert_eq!(
            detect_reverse_signal("[[RALPH:CONTINUE]]"),
            ReverseSignal::Continue
        );
    }

    // ========== Question handling tests ==========

    use tempfile::TempDir;

    fn create_temp_dir() -> TempDir {
        tempfile::tempdir().expect("Failed to create temp dir")
    }

    #[test]
    fn test_read_question_success() {
        let dir = create_temp_dir();
        let content = "# Investigation Question\n\nWhy does auth fail?";
        std::fs::write(dir.path().join("QUESTION.md"), content).unwrap();

        let result = read_question(dir.path()).unwrap();
        assert_eq!(result, content);
    }

    #[test]
    fn test_read_question_file_not_found() {
        let dir = create_temp_dir();
        let result = read_question(dir.path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("failed to read"));
    }

    #[test]
    fn test_create_question_template() {
        let dir = create_temp_dir();
        create_question_template(dir.path()).unwrap();

        let content = std::fs::read_to_string(dir.path().join("QUESTION.md")).unwrap();
        assert!(content.contains("# Investigation Question"));
        assert!(content.contains("Describe what you want to investigate"));
    }

    #[test]
    fn test_create_question_template_overwrites() {
        let dir = create_temp_dir();
        std::fs::write(dir.path().join("QUESTION.md"), "old content").unwrap();

        create_question_template(dir.path()).unwrap();

        let content = std::fs::read_to_string(dir.path().join("QUESTION.md")).unwrap();
        assert!(!content.contains("old content"));
        assert!(content.contains("# Investigation Question"));
    }

    #[test]
    fn test_write_question() {
        let dir = create_temp_dir();
        let question = "Why does the cache fail after 5 minutes?";

        write_question(dir.path(), question).unwrap();

        let content = std::fs::read_to_string(dir.path().join("QUESTION.md")).unwrap();
        assert!(content.contains("# Investigation Question"));
        assert!(content.contains(question));
        assert!(content.contains("## Context (Optional)"));
    }

    #[test]
    fn test_write_question_multiline() {
        let dir = create_temp_dir();
        let question = "Why does the auth fail?\n\n- Happens on OAuth users\n- Only in production";

        write_question(dir.path(), question).unwrap();

        let content = std::fs::read_to_string(dir.path().join("QUESTION.md")).unwrap();
        assert!(content.contains("# Investigation Question"));
        assert!(content.contains("Happens on OAuth users"));
        assert!(content.contains("Only in production"));
    }

    #[test]
    fn test_write_question_overwrites() {
        let dir = create_temp_dir();
        std::fs::write(dir.path().join("QUESTION.md"), "old question").unwrap();

        write_question(dir.path(), "new question").unwrap();

        let content = std::fs::read_to_string(dir.path().join("QUESTION.md")).unwrap();
        assert!(!content.contains("old question"));
        assert!(content.contains("new question"));
    }

    #[test]
    fn test_write_then_read_question() {
        let dir = create_temp_dir();
        let question = "What causes the memory leak?";

        write_question(dir.path(), question).unwrap();
        let content = read_question(dir.path()).unwrap();

        assert!(content.contains(question));
    }

    #[test]
    fn test_question_with_special_characters() {
        let dir = create_temp_dir();
        let question = "Why does `fn foo<T>()` fail with error \"E0277\"?";

        write_question(dir.path(), question).unwrap();
        let content = read_question(dir.path()).unwrap();

        assert!(content.contains(question));
    }

    #[test]
    fn test_question_with_unicode() {
        let dir = create_temp_dir();
        let question = "为什么缓存在5分钟后失败？";

        write_question(dir.path(), question).unwrap();
        let content = read_question(dir.path()).unwrap();

        assert!(content.contains(question));
    }
}
