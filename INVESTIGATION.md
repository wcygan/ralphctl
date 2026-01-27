# Investigation Log

**Question:** How is the reverse command implemented?
**Started:** 2026-01-27
**Status:** Complete

## Hypothesis 1: Reverse command has a dedicated module

- [x] Searched for "reverse" in codebase — Found `src/reverse.rs` module and `Command::Reverse` in `src/main.rs`
- [x] Examined `src/reverse.rs` — Contains `ReverseSignal` enum, signal detection functions, and question handling utilities
- [x] Examined `src/main.rs:740-853` — Contains `reverse_cmd()` async function that orchestrates the investigation loop
- **Result:** Confirmed

## Hypothesis 2: Uses existing forward mode infrastructure

- [x] Checked `src/run.rs` reuse — `spawn_claude()`, `log_iteration()`, `prompt_continue()`, `detect_blocked_signal()` are reused
- [x] Checked file constants — `src/files.rs` defines reverse-specific file constants (QUESTION.md, INVESTIGATION.md, FINDINGS.md, REVERSE_PROMPT.md)
- **Result:** Confirmed — shares subprocess spawning and logging with forward mode

## Hypothesis 3: Has distinct signal handling

- [x] Examined `ReverseSignal` enum — Has 5 variants: Continue, Found, Inconclusive, Blocked, NoSignal
- [x] Examined `detect_reverse_signal()` — Priority order: BLOCKED → FOUND → INCONCLUSIVE → CONTINUE
- [x] Compared to forward mode — Forward mode only has CONTINUE, DONE, and BLOCKED; reverse has FOUND and INCONCLUSIVE instead of DONE
- **Result:** Confirmed

## Hypothesis 4: Template is embedded in binary

- [x] Examined `src/templates.rs:22-24` — `EMBEDDED_REVERSE_PROMPT` uses `include_str!("../templates/REVERSE_PROMPT.md")`
- [x] Read `templates/REVERSE_PROMPT.md` — Contains investigation protocol with QUESTION.md, INVESTIGATION.md, FINDINGS.md workflow
- **Result:** Confirmed — unlike forward mode templates fetched from GitHub

## Key Findings

1. **Module Structure**: Reverse mode is implemented in `src/reverse.rs` with the main command function `reverse_cmd()` in `src/main.rs`
2. **CLI Definition**: `Command::Reverse` variant in `src/main.rs:183-198` with optional question argument, `--max-iterations`, `--pause`, and `--model` flags
3. **Question Handling**: Three scenarios: argument provided (writes QUESTION.md), QUESTION.md exists (uses it), neither (creates template and exits)
4. **Investigation Loop**: Reuses `run::spawn_claude()` to execute claude with the embedded REVERSE_PROMPT.md template piped via stdin
5. **Signal Detection**: `detect_reverse_signal()` parses Claude output for magic strings like `[[RALPH:FOUND:<summary>]]`
6. **Exit Codes**: 0=Found, 2=MaxIterations, 3=Blocked, 4=Inconclusive, 130=Interrupted
7. **Test Coverage**: Comprehensive integration tests in `tests/reverse.rs` (1700+ lines) using mock claude scripts
