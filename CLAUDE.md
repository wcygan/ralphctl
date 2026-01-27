# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`ralphctl` is a Rust CLI tool for managing Ralph Loop workflows—autonomous development sessions driven by Claude. It orchestrates `claude -p` subprocess calls to execute iterative development tasks defined in markdown files.

## Build & Test Commands

```bash
cargo build              # Build debug binary
cargo test               # Run all tests
cargo test <test_name>   # Run specific test
cargo clippy             # Lint (must pass with no warnings)
cargo fmt                # Format code
cargo run -- <subcommand>  # Run with args (e.g., cargo run -- status)
```

## Architecture

### Module Structure

- `main.rs` - CLI entry point, command dispatch, clap derive definitions
- `cli.rs` - External CLI detection (claude binary in PATH)
- `run.rs` - Core ralph loop execution: subprocess spawning, real-time streaming, magic string detection
- `parser.rs` - Markdown checkbox parsing for progress tracking (`- [ ]` / `- [x]`)
- `files.rs` - Ralph file constants and discovery (`SPEC.md`, `IMPLEMENTATION_PLAN.md`, `PROMPT.md`, `ralph.log`)
- `templates.rs` - GitHub template fetching with XDG-compliant cache fallback
- `error.rs` - Unix-style error formatting and exit codes

### Key Patterns

**Subprocess execution**: `run::spawn_claude()` pipes PROMPT.md to `claude -p` via stdin, streams stdout/stderr in real-time using threads, and captures output for magic string detection.

**Template caching**: Network-first strategy—fetch from GitHub, cache locally, fall back to cache on network failure. Cache location: `~/.cache/ralphctl/templates/` (Linux) or `~/Library/Caches/ralphctl/templates/` (macOS).

**Magic strings**: Loop termination signals embedded in Claude output:
- `[[RALPH:DONE]]` - All tasks complete
- `[[RALPH:BLOCKED:<reason>]]` - Cannot proceed

### Exit Codes

```
0   - Success
1   - General error
2   - Max iterations reached
130 - Interrupted (Ctrl+C)
```

## Ralph Workflow Files

Commands operate on these files in the working directory:

| File | Purpose |
|------|---------|
| `SPEC.md` | Project specification |
| `IMPLEMENTATION_PLAN.md` | Task list with checkboxes |
| `PROMPT.md` | Orchestration prompt piped to Claude |
| `ralph.log` | Iteration output log (append mode) |

## Code Style

- Terse Unix-style errors: `error: claude not found in PATH`
- `anyhow::Result` for fallible functions
- Early returns over deep nesting
- No emojis in code or output
