# Implementation Plan

**Generated from:** SPEC.md
**Status:** In Progress
**Last Updated:** 2026-01-27

---

## Phase 1: Project Setup (Complete)

- [x] Initialize Cargo project with `ralphctl` binary target and required dependencies (clap, anyhow, tokio, reqwest, dirs, regex)
- [x] Set up clap with derive macros and subcommand structure (init, run, status, clean)
- [x] Add basic error handling module with anyhow integration
- [x] Create GitHub Actions CI workflow for tests and clippy on PRs and main

## Phase 2: Status Command (Complete)

- [x] Implement markdown checkbox parser using regex to count `- [ ]` and `- [x]`
- [x] Implement `status` subcommand that reads IMPLEMENTATION_PLAN.md and counts tasks
- [x] Add Unicode progress bar rendering (`[████████░░░░] 60% (12/20 tasks)`)
- [x] Add error handling for missing IMPLEMENTATION_PLAN.md
- [x] Write unit tests for checkbox parsing with various markdown edge cases

## Phase 3: Clean Command (Complete)

- [x] Implement file existence checking for ralph files (SPEC.md, IMPLEMENTATION_PLAN.md, PROMPT.md, ralph.log)
- [x] Implement `clean` subcommand with `[y/N]` confirmation prompt
- [x] Add `--force` flag to skip confirmation
- [x] Handle "no files found" case with success message
- [x] Write integration tests for clean command with fixture files

## Phase 4: Init Command (Complete)

- [x] Implement `claude` CLI detection using `which` command
- [x] Implement template fetching from GitHub raw content URLs using reqwest
- [x] Implement XDG-compliant cache directory resolution using dirs crate
- [x] Implement cache-first-then-fetch logic with network fallback to cache
- [x] Implement `init` subcommand that writes SPEC.md, IMPLEMENTATION_PLAN.md, PROMPT.md
- [x] Add `--force` flag to overwrite existing files
- [x] Add file existence check that errors without --force
- [x] Write integration tests for init command with mock HTTP responses

## Phase 5: Run Command Core (Complete)

- [x] Implement PROMPT.md file reading
- [x] Implement subprocess spawning for `claude -p` with stdin piping
- [x] Implement real-time stdout/stderr streaming (pass-through)
- [x] Implement iteration header printing (`=== Iteration N starting ===`)
- [x] Implement magic string detection for `[[RALPH:DONE]]`
- [x] Implement magic string detection for `[[RALPH:BLOCKED:<reason>]]` with reason extraction
- [x] Implement `--max-iterations` flag with default of 50
- [x] Implement pre-run validation for required files (PROMPT.md, SPEC.md, IMPLEMENTATION_PLAN.md)
- [x] Write integration tests for run command with mock claude output

## Phase 6: Run Command Enhancements (Complete)

- [x] Implement ralph.log file writing in append mode with structured sections
- [x] Implement Ctrl+C signal handling that forwards to child process
- [x] Implement interrupt summary printing (`Interrupted after N iterations. X/Y tasks complete.`)
- [x] Implement `--pause` flag with `Continue? [Y/n]` prompt
- [x] Implement fallback user prompt when no magic string detected after iteration
- [x] Write tests for logging format

## Phase 7: Distribution (Complete)

- [x] Add Cross.toml configuration for cross-rs
- [x] Add GitHub Actions release workflow triggered by `v*` tags
- [x] Configure release matrix for macOS (arm64, x86_64) and Linux (x86_64, arm64)
- [x] Test cross-compilation locally for at least one non-native target

## Phase 8: Documentation & Templates (Complete)

- [x] Create templates/SPEC.md with section headers only
- [x] Create templates/IMPLEMENTATION_PLAN.md with section headers only
- [x] Create templates/PROMPT.md with full ralph orchestration prompt
- [x] Write README.md with installation instructions and usage examples
- [x] Polish --help text in clap with clear descriptions and examples
- [x] Create CHANGELOG.md for v0.1.0 release

---

## Phase 9: Reverse Mode Foundation

- [ ] Add reverse mode file constants to `files.rs` (QUESTION_FILE, INVESTIGATION_FILE, FINDINGS_FILE, REVERSE_PROMPT_FILE)
- [ ] Add `exit::INCONCLUSIVE = 4` constant to `error.rs`
- [ ] Add REVERSE_FILES array and `find_existing_reverse_files()` function to `files.rs`
- [ ] Add `find_archivable_reverse_files()` function to `files.rs`
- [ ] Write unit tests for reverse file discovery functions

## Phase 10: Reverse Mode Signal Detection

- [ ] Create `reverse.rs` module with `ReverseSignal` enum (Continue, Found, Inconclusive, Blocked, NoSignal)
- [ ] Implement `detect_reverse_signal()` function with BLOCKED → FOUND → INCONCLUSIVE → CONTINUE priority
- [ ] Add signal marker constants (RALPH_FOUND_PREFIX, RALPH_INCONCLUSIVE_PREFIX)
- [ ] Write unit tests for all signal detection scenarios (exact match, with whitespace, inline rejection)
- [ ] Write tests for signal priority (BLOCKED → FOUND → INCONCLUSIVE → CONTINUE)

## Phase 11: Reverse Mode Question Handling

- [ ] Implement `read_question()` function in `reverse.rs`
- [ ] Implement `create_question_template()` function with minimal placeholder content
- [ ] Implement `write_question()` function for CLI argument → file
- [ ] Write unit tests for question reading and template creation

## Phase 12: Reverse Mode Template

- [ ] Create `templates/REVERSE_PROMPT.md` with investigation loop instructions
- [ ] Add REVERSE_PROMPT.md to `templates.rs` fetching logic
- [ ] Update `get_all_templates()` to NOT include REVERSE_PROMPT.md (it's fetched separately)
- [ ] Add `get_reverse_template()` function for on-demand fetching
- [ ] Write integration test for reverse template fetching and caching

## Phase 13: Reverse Mode CLI

- [ ] Add `Command::Reverse` variant to CLI with question argument, --max-iterations (default 100), --pause, --model flags
- [ ] Implement `reverse_cmd()` function in `main.rs`
- [ ] Implement question setup logic (arg provided vs QUESTION.md exists vs create template)
- [ ] Implement reverse iteration loop using `spawn_claude()` with REVERSE_PROMPT.md
- [ ] Implement signal handling: CONTINUE (loop), FOUND (exit 0), INCONCLUSIVE (exit 4), BLOCKED (exit 3)
- [ ] Add comprehensive --help text with examples and exit code documentation

## Phase 14: Reverse Mode Integration

- [ ] Update `find_existing_ralph_files()` in `files.rs` to include reverse files
- [ ] Update `clean_cmd()` to handle reverse files (same confirmation UX)
- [ ] Update `find_archivable_files()` to include QUESTION.md, INVESTIGATION.md, FINDINGS.md
- [ ] Update `archive_cmd()` to handle reverse files (archive and reset)
- [ ] Update `generate_blank_content()` for QUESTION.md reset
- [ ] Write integration tests for clean command with reverse files
- [ ] Write integration tests for archive command with reverse files

## Phase 15: Reverse Mode Testing

- [ ] Write integration test for `ralphctl reverse "question"` happy path
- [ ] Write integration test for `ralphctl reverse` without args (QUESTION.md exists)
- [ ] Write integration test for `ralphctl reverse` without args (creates template)
- [ ] Write integration test for CONTINUE signal (loop continues)
- [ ] Write integration test for FOUND signal termination
- [ ] Write integration test for INCONCLUSIVE signal termination
- [ ] Write integration test for BLOCKED signal termination
- [ ] Write integration test for max iterations reached
- [ ] Write integration test for --pause flag in reverse mode

## Phase 16: Documentation Update

- [ ] Update README.md with Reverse Mode section and examples
- [ ] Add reverse mode examples to CLI --help text
- [ ] Update CHANGELOG.md with reverse mode feature

---

## Task Guidelines

Each task should be:
- **Atomic**: Completable in one focused session
- **Testable**: Has clear verification criteria
- **Independent**: Minimal dependencies on incomplete tasks

## Progress Tracking

Format: `- [x]` = complete, `- [ ]` = pending, `- [>]` = in progress
