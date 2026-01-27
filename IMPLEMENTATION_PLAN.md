# Implementation Plan

**Generated from:** SPEC.md
**Status:** In Progress
**Last Updated:** 2026-01-27

---

## Phase 1: Project Setup

- [x] Initialize Cargo project with `ralphctl` binary target and required dependencies (clap, anyhow, tokio, reqwest, dirs, regex)
- [x] Set up clap with derive macros and subcommand structure (init, run, status, clean)
- [x] Add basic error handling module with anyhow integration
- [x] Create GitHub Actions CI workflow for tests and clippy on PRs and main

## Phase 2: Status Command

- [x] Implement markdown checkbox parser using regex to count `- [ ]` and `- [x]`
- [x] Implement `status` subcommand that reads IMPLEMENTATION_PLAN.md and counts tasks
- [x] Add Unicode progress bar rendering (`[████████░░░░] 60% (12/20 tasks)`)
- [x] Add error handling for missing IMPLEMENTATION_PLAN.md
- [x] Write unit tests for checkbox parsing with various markdown edge cases

## Phase 3: Clean Command

- [x] Implement file existence checking for ralph files (SPEC.md, IMPLEMENTATION_PLAN.md, PROMPT.md, ralph.log)
- [x] Implement `clean` subcommand with `[y/N]` confirmation prompt
- [x] Add `--force` flag to skip confirmation
- [x] Handle "no files found" case with success message
- [x] Write integration tests for clean command with fixture files

## Phase 4: Init Command

- [x] Implement `claude` CLI detection using `which` command
- [x] Implement template fetching from GitHub raw content URLs using reqwest
- [x] Implement XDG-compliant cache directory resolution using dirs crate
- [x] Implement cache-first-then-fetch logic with network fallback to cache
- [x] Implement `init` subcommand that writes SPEC.md, IMPLEMENTATION_PLAN.md, PROMPT.md
- [x] Add `--force` flag to overwrite existing files
- [x] Add file existence check that errors without --force
- [x] Write integration tests for init command with mock HTTP responses

## Phase 5: Run Command Core

- [x] Implement PROMPT.md file reading
- [x] Implement subprocess spawning for `claude -p` with stdin piping
- [x] Implement real-time stdout/stderr streaming (pass-through)
- [x] Implement iteration header printing (`=== Iteration N starting ===`)
- [x] Implement magic string detection for `[[RALPH:DONE]]`
- [x] Implement magic string detection for `[[RALPH:BLOCKED:<reason>]]` with reason extraction
- [x] Implement `--max-iterations` flag with default of 50
- [x] Implement pre-run validation for required files (PROMPT.md, SPEC.md, IMPLEMENTATION_PLAN.md)
- [x] Write integration tests for run command with mock claude output

## Phase 6: Run Command Enhancements

- [x] Implement ralph.log file writing in append mode with structured sections
- [x] Implement Ctrl+C signal handling that forwards to child process
- [x] Implement interrupt summary printing (`Interrupted after N iterations. X/Y tasks complete.`)
- [x] Implement `--pause` flag with `Continue? [Y/n]` prompt
- [x] Implement fallback user prompt when no magic string detected after iteration
- [x] Write tests for logging format

## Phase 7: Distribution

- [x] Add Cross.toml configuration for cross-rs
- [x] Add GitHub Actions release workflow triggered by `v*` tags
- [x] Configure release matrix for macOS (arm64, x86_64) and Linux (x86_64, arm64)
- [x] Test cross-compilation locally for at least one non-native target

## Phase 8: Documentation & Templates

- [x] Create templates/SPEC.md with section headers only
- [x] Create templates/IMPLEMENTATION_PLAN.md with section headers only
- [x] Create templates/PROMPT.md with full ralph orchestration prompt
- [x] Write README.md with installation instructions and usage examples
- [x] Polish --help text in clap with clear descriptions and examples
- [ ] Create CHANGELOG.md for v0.1.0 release

---

## Task Guidelines

Each task should be:
- **Atomic**: Completable in one focused session
- **Testable**: Has clear verification criteria
- **Independent**: Minimal dependencies on incomplete tasks

## Progress Tracking

Format: `- [x]` = complete, `- [ ]` = pending, `- [>]` = in progress
