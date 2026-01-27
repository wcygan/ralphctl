# Investigation Findings

**Question:** How is the reverse command implemented?
**Status:** Answered
**Date:** 2026-01-27

## Summary

The `ralphctl reverse` command is implemented as an autonomous investigation loop for answering questions about codebases. It consists of three main components: (1) a dedicated `src/reverse.rs` module for signal detection and question handling, (2) the `reverse_cmd()` async function in `src/main.rs` that orchestrates the loop, and (3) an embedded prompt template in `templates/REVERSE_PROMPT.md` compiled into the binary.

Unlike forward mode which builds software by completing tasks, reverse mode operates read-only and produces investigation reports. It reuses the forward mode's subprocess spawning infrastructure (`run::spawn_claude()`) while defining its own signal protocol with FOUND, INCONCLUSIVE, BLOCKED, and CONTINUE outcomes.

## Evidence

### Module: `src/reverse.rs` (lines 1-172)

Provides reverse-mode-specific functionality:

```rust
/// Reverse mode signal types.
pub enum ReverseSignal {
    Continue,                    // Still investigating
    Found(String),               // Question answered (summary)
    Inconclusive(String),        // Cannot determine (reason)
    Blocked(String),             // Cannot proceed (reason)
    NoSignal,                    // No signal detected
}

/// Detect reverse mode signals in output.
/// Detection priority: BLOCKED → FOUND → INCONCLUSIVE → CONTINUE
pub fn detect_reverse_signal(output: &str) -> ReverseSignal
```

Also provides question handling utilities:
- `read_question()` — Reads QUESTION.md
- `write_question()` — Creates QUESTION.md with formatted question
- `create_question_template()` — Creates placeholder QUESTION.md

### Command Function: `src/main.rs:740-853`

The `reverse_cmd()` function implements the investigation loop:

1. **Question Setup** (lines 751-764):
   - If argument provided → writes QUESTION.md
   - If QUESTION.md exists → uses it
   - Otherwise → creates template and exits with code 1

2. **Template Loading** (lines 771-775):
   - Gets embedded template via `templates::get_reverse_template()`
   - Writes REVERSE_PROMPT.md to working directory for reference

3. **Investigation Loop** (lines 786-845):
   - Iterates up to `max_iterations` (default: 100)
   - Calls `run::spawn_claude(&prompt, model, interrupt_flag)` each iteration
   - Logs output to `ralph.log`
   - Detects signals and responds:
     - `Blocked` → exits code 3
     - `Found` → prints summary, exits code 0
     - `Inconclusive` → exits code 4
     - `Continue` → proceeds to next iteration
     - `NoSignal` → prompts user for action

### CLI Definition: `src/main.rs:183-198`

```rust
Reverse {
    /// The investigation question (reads from QUESTION.md if omitted)
    question: Option<String>,

    /// Maximum iterations before stopping
    #[arg(long, default_value = "100", value_name = "N")]
    max_iterations: u32,

    /// Prompt for confirmation before each iteration
    #[arg(long)]
    pause: bool,

    /// Claude model to use
    #[arg(long, value_name = "MODEL")]
    model: Option<String>,
}
```

### Embedded Template: `src/templates.rs:22-24`

```rust
/// Embedded reverse mode prompt template (compiled into binary).
const EMBEDDED_REVERSE_PROMPT: &str = include_str!("../templates/REVERSE_PROMPT.md");
```

The template (`templates/REVERSE_PROMPT.md`, 160 lines) defines:
- Context files: QUESTION.md, INVESTIGATION.md
- Investigation protocol: Orient → Investigate → Update State → Report & Signal
- Signal format: `[[RALPH:FOUND:<summary>]]`, `[[RALPH:INCONCLUSIVE:<reason>]]`, etc.
- Output files: INVESTIGATION.md (log), FINDINGS.md (final report)

### File Constants: `src/files.rs:15-37`

```rust
pub const QUESTION_FILE: &str = "QUESTION.md";
pub const INVESTIGATION_FILE: &str = "INVESTIGATION.md";
pub const FINDINGS_FILE: &str = "FINDINGS.md";
pub const REVERSE_PROMPT_FILE: &str = "REVERSE_PROMPT.md";

pub const ARCHIVABLE_REVERSE_FILES: &[&str] = &[QUESTION_FILE, INVESTIGATION_FILE, FINDINGS_FILE];
```

### Exit Codes: `src/main.rs:167-181` (help text)

| Code | Meaning |
|------|---------|
| 0 | Found (question answered) |
| 2 | Max iterations reached |
| 3 | Blocked (requires human intervention) |
| 4 | Inconclusive (cannot determine answer) |
| 130 | Interrupted (Ctrl+C) |

## Recommendations

1. **Understanding the flow**: To trace execution, start at `reverse_cmd()` in `src/main.rs:740`, which calls into `reverse.rs` for signal detection and `run.rs` for subprocess management.

2. **Adding new signals**: Extend `ReverseSignal` enum in `src/reverse.rs` and update `detect_reverse_signal()` with appropriate priority.

3. **Modifying the prompt**: Edit `templates/REVERSE_PROMPT.md`—changes are compiled into the binary, so rebuild is required.

4. **Testing**: Use mock claude scripts (see `tests/reverse.rs`) to simulate different signal outputs without running actual Claude.

## Investigation Path

1. Searched codebase for "reverse" and "Reverse" patterns
2. Read `src/reverse.rs` (800 lines including tests) — signal types, detection logic, question utilities
3. Read `src/main.rs:740-862` — `reverse_cmd()` function and interrupt summary helper
4. Read `src/main.rs:180-245` — CLI definition and command dispatch
5. Read `src/templates.rs:1-30, 200-215` — template constants and `get_reverse_template()`
6. Read `templates/REVERSE_PROMPT.md` (160 lines) — full prompt template
7. Read `src/files.rs` file constants — reverse mode file names
8. Read `tests/reverse.rs` (1700 lines) — comprehensive integration test suite
