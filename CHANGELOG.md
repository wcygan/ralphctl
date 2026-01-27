# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-01-27

### Added

#### Commands

- **`ralphctl reverse`**: Autonomous investigation mode for analyzing codebases
  - Investigate codebases to answer questions—diagnosing bugs, understanding legacy code, or mapping dependencies
  - Question argument: `ralphctl reverse "Why does auth fail?"` creates QUESTION.md and starts investigation
  - No-arg behavior: uses existing QUESTION.md or creates template with instructions
  - REVERSE_PROMPT.md fetched from GitHub and cached (like PROMPT.md)
  - New signal protocol:
    - `[[RALPH:FOUND:<summary>]]` for answered questions (exit 0)
    - `[[RALPH:INCONCLUSIVE:<why>]]` for exhausted investigations (exit 4)
    - `[[RALPH:CONTINUE]]` and `[[RALPH:BLOCKED:<reason>]]` shared with forward mode
  - `--max-iterations` flag (default: 100) for investigation depth
  - `--pause` flag for interactive confirmation between iterations
  - `--model` flag to select Claude model
  - Investigation logged to ralph.log with same structured format

#### Files

- **QUESTION.md**: The investigation question (created by user or `ralphctl reverse`)
- **INVESTIGATION.md**: Running log of hypotheses with checkboxes (maintained by Claude)
- **FINDINGS.md**: Final synthesized report (written by Claude on completion)
- **REVERSE_PROMPT.md**: Investigation loop instructions (fetched from GitHub)

#### Exit Codes

- `4`: Inconclusive (investigation exhausted without definitive answer)

### Changed

- `ralphctl clean` now removes reverse mode files (QUESTION.md, INVESTIGATION.md, FINDINGS.md, REVERSE_PROMPT.md)
- `ralphctl archive` now archives reverse mode files (QUESTION.md, INVESTIGATION.md, FINDINGS.md)

---

## [0.1.0] - 2026-01-27

Initial release of `ralphctl`, a CLI tool for managing Ralph Loop workflows.

### Added

#### Commands

- **`ralphctl init`**: Scaffold ralph loop files from GitHub templates
  - Fetches SPEC.md, IMPLEMENTATION_PLAN.md, and PROMPT.md from GitHub
  - XDG-compliant template caching for offline use
  - `--force` flag to overwrite existing files
  - Verifies `claude` CLI is available in PATH before proceeding

- **`ralphctl interview`**: AI-guided interview to create SPEC.md and plan
  - Interactive session with Claude to define project specifications
  - `--model` flag to select Claude model

- **`ralphctl run`**: Execute the autonomous development loop
  - Pipes PROMPT.md to `claude -p` via stdin
  - Real-time stdout/stderr streaming
  - Magic string detection: `[[RALPH:DONE]]` for completion, `[[RALPH:BLOCKED:<reason>]]` for blockers
  - `--max-iterations` flag (default: 50) to limit loop iterations
  - `--pause` flag for interactive confirmation between iterations
  - `--model` flag to select Claude model
  - Structured logging to ralph.log with iteration headers
  - Graceful Ctrl+C handling with interrupt summary
  - Fallback user prompt when no magic string detected

- **`ralphctl status`**: Display progress bar with task completion stats
  - Parses IMPLEMENTATION_PLAN.md for checkbox status
  - Unicode progress bar: `[████████░░░░] 60% (12/20 tasks)`

- **`ralphctl archive`**: Archive completed ralph loop files
  - Saves spec and plan to `.ralphctl/archive/<timestamp>/`
  - Resets files to blank templates
  - `--force` flag to skip confirmation

- **`ralphctl clean`**: Remove ralph loop files
  - Interactive `[y/N]` confirmation prompt
  - `--force` flag to skip confirmation
  - Graceful handling when no files exist

#### Infrastructure

- Cross-platform support for macOS (arm64, x86_64) and Linux (x86_64, arm64)
- GitHub Actions CI workflow for tests and clippy on PRs and main
- GitHub Actions release workflow triggered by `v*` tags
- Static binary builds using musl for Linux targets
- Release artifacts with SHA256 checksums

#### Exit Codes

- `0`: Success
- `1`: General error
- `2`: Max iterations reached
- `3`: Blocked (requires human intervention)
- `130`: Interrupted (Ctrl+C)

### Dependencies

- `clap` (4.5): CLI argument parsing with derive macros
- `tokio` (1.43): Async runtime for subprocess management
- `reqwest` (0.12): HTTP client for template fetching
- `anyhow` (1.0): Error handling with context
- `dirs` (5.0): XDG-compliant directory resolution
- `regex` (1.10): Markdown checkbox parsing
- `chrono` (0.4): Timestamp formatting
- `ctrlc` (3.5): Signal handling for graceful interruption
- `nix` (0.31): Unix signal forwarding to child processes

[0.2.0]: https://github.com/wcygan/ralphctl/releases/tag/v0.2.0
[0.1.0]: https://github.com/wcygan/ralphctl/releases/tag/v0.1.0
