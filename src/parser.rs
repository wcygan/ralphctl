//! Markdown parsing utilities for ralphctl.
//!
//! Provides checkbox counting for IMPLEMENTATION_PLAN.md progress tracking.

#![allow(dead_code)] // Used by status command (next task)

use regex::Regex;

/// Result of parsing checkboxes from markdown content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCount {
    /// Number of completed tasks (`- [x]`)
    pub completed: usize,
    /// Total number of tasks (`- [ ]` + `- [x]`)
    pub total: usize,
}

impl TaskCount {
    /// Create a new TaskCount with the given values.
    pub fn new(completed: usize, total: usize) -> Self {
        Self { completed, total }
    }

    /// Calculate completion percentage (0-100).
    pub fn percentage(&self) -> u8 {
        if self.total == 0 {
            return 0;
        }
        ((self.completed as f64 / self.total as f64) * 100.0).round() as u8
    }

    /// Render a Unicode progress bar with stats.
    ///
    /// Format: `[████████░░░░] 60% (12/20 tasks)`
    pub fn render_progress_bar(&self) -> String {
        const BAR_WIDTH: usize = 12;
        const FILLED: char = '█';
        const EMPTY: char = '░';

        let pct = self.percentage();
        let filled_count = if self.total == 0 {
            0
        } else {
            (self.completed * BAR_WIDTH) / self.total
        };
        let empty_count = BAR_WIDTH - filled_count;

        let filled: String = std::iter::repeat_n(FILLED, filled_count).collect();
        let empty: String = std::iter::repeat_n(EMPTY, empty_count).collect();

        format!(
            "[{}{}] {}% ({}/{} tasks)",
            filled, empty, pct, self.completed, self.total
        )
    }
}

/// Count completed and total checkboxes in markdown content.
///
/// Matches standard markdown checkbox syntax:
/// - `- [ ]` for incomplete tasks
/// - `- [x]` or `- [X]` for complete tasks
///
/// Counting is flat (no nesting weight).
pub fn count_checkboxes(content: &str) -> TaskCount {
    // Regex matches:
    // - `- [ ]` (incomplete, whitespace inside brackets)
    // - `- [x]` or `- [X]` (complete)
    // Anchored to line start with optional leading whitespace
    let checkbox_re = Regex::new(r"(?m)^\s*-\s*\[([ xX])\]").unwrap();

    let mut completed = 0;
    let mut total = 0;

    for cap in checkbox_re.captures_iter(content) {
        total += 1;
        if let Some(mark) = cap.get(1) {
            let c = mark.as_str();
            if c == "x" || c == "X" {
                completed += 1;
            }
        }
    }

    TaskCount::new(completed, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_content() {
        let count = count_checkboxes("");
        assert_eq!(count, TaskCount::new(0, 0));
        assert_eq!(count.percentage(), 0);
    }

    #[test]
    fn test_no_checkboxes() {
        let content = "# Heading\n\nSome text without checkboxes.\n\n- Regular list item";
        let count = count_checkboxes(content);
        assert_eq!(count, TaskCount::new(0, 0));
    }

    #[test]
    fn test_incomplete_only() {
        let content = "- [ ] Task 1\n- [ ] Task 2\n- [ ] Task 3";
        let count = count_checkboxes(content);
        assert_eq!(count, TaskCount::new(0, 3));
        assert_eq!(count.percentage(), 0);
    }

    #[test]
    fn test_complete_only() {
        let content = "- [x] Task 1\n- [x] Task 2";
        let count = count_checkboxes(content);
        assert_eq!(count, TaskCount::new(2, 2));
        assert_eq!(count.percentage(), 100);
    }

    #[test]
    fn test_mixed_tasks() {
        let content = "- [x] Done\n- [ ] Pending\n- [x] Also done\n- [ ] Another pending";
        let count = count_checkboxes(content);
        assert_eq!(count, TaskCount::new(2, 4));
        assert_eq!(count.percentage(), 50);
    }

    #[test]
    fn test_uppercase_x() {
        let content = "- [X] Uppercase mark\n- [x] Lowercase mark";
        let count = count_checkboxes(content);
        assert_eq!(count, TaskCount::new(2, 2));
    }

    #[test]
    fn test_with_indentation() {
        let content = "  - [ ] Indented incomplete\n    - [x] More indented complete";
        let count = count_checkboxes(content);
        assert_eq!(count, TaskCount::new(1, 2));
    }

    #[test]
    fn test_with_surrounding_content() {
        let content = r#"
# Implementation Plan

## Phase 1

- [x] Initialize project
- [x] Set up CI

## Phase 2

- [ ] Implement feature A
- [ ] Implement feature B
- [x] Write tests

Some other text here.
"#;
        let count = count_checkboxes(content);
        assert_eq!(count, TaskCount::new(3, 5));
        assert_eq!(count.percentage(), 60);
    }

    #[test]
    fn test_percentage_rounding() {
        // 1/3 = 33.33...% should round to 33
        let count = TaskCount::new(1, 3);
        assert_eq!(count.percentage(), 33);

        // 2/3 = 66.66...% should round to 67
        let count = TaskCount::new(2, 3);
        assert_eq!(count.percentage(), 67);
    }

    #[test]
    fn test_checkbox_not_at_line_start_ignored() {
        // Checkboxes embedded in text (not at line start) should still match
        // because the regex allows leading whitespace
        let content = "Text before - [ ] inline checkbox";
        let count = count_checkboxes(content);
        // This actually shouldn't match because "Text before" is not whitespace
        assert_eq!(count, TaskCount::new(0, 0));
    }

    #[test]
    fn test_real_implementation_plan_format() {
        let content = r#"
# Implementation Plan

**Generated from:** SPEC.md
**Status:** In Progress

---

## Phase 1: Project Setup

- [x] Initialize Cargo project
- [x] Set up clap structure

## Phase 2: Core Features

- [ ] Implement status command
- [ ] Implement clean command
"#;
        let count = count_checkboxes(content);
        assert_eq!(count, TaskCount::new(2, 4));
        assert_eq!(count.percentage(), 50);
    }

    #[test]
    fn test_progress_bar_empty() {
        let count = TaskCount::new(0, 0);
        assert_eq!(count.render_progress_bar(), "[░░░░░░░░░░░░] 0% (0/0 tasks)");
    }

    #[test]
    fn test_progress_bar_zero_percent() {
        let count = TaskCount::new(0, 10);
        assert_eq!(
            count.render_progress_bar(),
            "[░░░░░░░░░░░░] 0% (0/10 tasks)"
        );
    }

    #[test]
    fn test_progress_bar_half() {
        let count = TaskCount::new(6, 12);
        assert_eq!(
            count.render_progress_bar(),
            "[██████░░░░░░] 50% (6/12 tasks)"
        );
    }

    #[test]
    fn test_progress_bar_full() {
        let count = TaskCount::new(20, 20);
        assert_eq!(
            count.render_progress_bar(),
            "[████████████] 100% (20/20 tasks)"
        );
    }

    #[test]
    fn test_progress_bar_60_percent() {
        let count = TaskCount::new(12, 20);
        assert_eq!(
            count.render_progress_bar(),
            "[███████░░░░░] 60% (12/20 tasks)"
        );
    }
}
