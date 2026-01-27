# CLAUDE.md

Guidance for Claude Code when working with this repository.

## Project Overview

`ralphctl` is a Rust CLI for managing Ralph Loop workflows—autonomous development sessions driven by Claude. It orchestrates `claude` subprocess calls to execute iterative development tasks defined in markdown files.

**Workflow**: `init → interview → run → verify → clean`

## Build & Test

```bash
cargo build                    # Build debug binary
cargo test                     # Run all tests (93 tests)
cargo clippy                   # Lint (must pass with no warnings)
cargo fmt                      # Format code
cargo run -- <command>         # Run with args
```

## CLI Commands

| Command | Description | Key Flags |
|---------|-------------|-----------|
| `init` | Scaffold ralph files from GitHub templates | `--force` |
| `interview` | AI-guided interview to create SPEC.md and plan | `--model` |
| `run` | Execute loop until done or blocked | `--max-iterations`, `--pause`, `--model` |
| `status` | Show progress bar from IMPLEMENTATION_PLAN.md | — |
| `clean` | Remove ralph loop files | `--force` |

## Architecture

### Module Structure

| Module | Purpose | Key Functions |
|--------|---------|---------------|
| `main.rs` | CLI entry, command dispatch | `interview_cmd()` (:178), `run_cmd()` (:143) |
| `cli.rs` | Claude binary detection | `claude_exists()` (:11) |
| `run.rs` | Loop execution, subprocess spawning | `spawn_claude()` (:106), `detect_done_signal()` (:98) |
| `parser.rs` | Checkbox parsing for progress | `count_checkboxes()` (:41), `render_progress_bar()` (:62) |
| `files.rs` | File constants and discovery | `find_existing_ralph_files()` (:26) |
| `templates.rs` | GitHub fetch with XDG cache | `get_all_templates()` (:40), `fetch_template()` (:94) |
| `error.rs` | Unix-style errors, exit codes | `die()` (:24), exit codes (:10-18) |

### Key Patterns

**Subprocess execution** (`run.rs:106-154`): `spawn_claude()` pipes PROMPT.md to `claude -p` via stdin, spawns threads for real-time stdout/stderr streaming, captures output for magic string detection.

**Interview mode** (`main.rs:178-316`): Launches `claude` interactively with `--system-prompt` containing Ralph Loop context and `--allowedTools` restricted to: AskUserQuestion, Read, Glob, Grep, Write, Edit.

**Template caching** (`templates.rs`): Network-first strategy—fetch from GitHub, cache locally, fall back to cache on failure. Cache: `~/.cache/ralphctl/templates/` (Linux) or `~/Library/Caches/ralphctl/templates/` (macOS).

**Magic strings** (`run.rs:65-104`): Loop termination signals in Claude output:
- `[[RALPH:DONE]]` — All tasks complete (implemented)
- `[[RALPH:BLOCKED:<reason>]]` — Cannot proceed (not yet implemented)

### Exit Codes

```
0   - Success
1   - General error
2   - Max iterations reached
3   - Blocked (requires human intervention)
130 - Interrupted (Ctrl+C)
```

## Project Structure

```
src/
├── main.rs          # CLI entry point
├── cli.rs           # Claude detection
├── run.rs           # Loop execution
├── parser.rs        # Checkbox parsing
├── files.rs         # File constants
├── templates.rs     # Template fetching
└── error.rs         # Error handling

templates/           # Source templates for init
├── SPEC.md
├── IMPLEMENTATION_PLAN.md
└── PROMPT.md

tests/               # Integration tests
├── init.rs
└── clean.rs
```

## Ralph Workflow Files

| File | Purpose | Created By |
|------|---------|------------|
| `SPEC.md` | Project specification | init, interview |
| `IMPLEMENTATION_PLAN.md` | Task list with checkboxes | init, interview |
| `PROMPT.md` | Orchestration prompt piped to Claude | init |
| `ralph.log` | Iteration output log | run |

## Code Style

- Terse Unix-style errors: `error: claude not found in PATH`
- `anyhow::Result` for fallible functions
- Early returns over deep nesting
- No emojis in code or output
- `#[allow(dead_code)]` with comments for staged development
