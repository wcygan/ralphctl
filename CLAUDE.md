# CLAUDE.md

Guidance for Claude Code when working with this repository.

## Project Overview

`ralphctl` is a Rust CLI for managing Ralph Loop workflows—autonomous development sessions driven by Claude. It orchestrates `claude` subprocess calls to execute iterative development tasks defined in markdown files.

**Workflow**: `init → interview → run → archive (or clean)`

## Build & Test

```bash
cargo build                    # Build debug binary
cargo test                     # Run all tests (127 tests)
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
| `archive` | Save spec/plan to `.ralphctl/archive/<timestamp>/`, reset to blank | `--force` |
| `clean` | Remove ralph loop files | `--force` |
| `update` | Install latest version from GitHub | — |

## Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing (derive macros) |
| `anyhow` | Error handling with context |
| `tokio` | Async runtime, subprocess spawning |
| `reqwest` | HTTP client for GitHub template fetching |
| `regex` | Checkbox pattern matching |
| `dirs` | XDG-compliant cache directory resolution |
| `chrono` | Timestamp generation for archives |
| `ctrlc` | Graceful Ctrl+C handling |
| `nix` | Unix signal handling |

## Architecture

### Module Structure

| Module | Purpose | Key Functions |
|--------|---------|---------------|
| `main.rs` | CLI entry, command dispatch | `run_cmd()`, `interview_cmd()`, `init_cmd()` |
| `cli.rs` | Claude binary detection | `claude_exists()` |
| `run.rs` | Loop execution, subprocess spawning | `spawn_claude()`, `detect_signal()`, `detect_blocked_signal()`, `log_iteration()`, `prompt_continue()` |
| `parser.rs` | Checkbox parsing for progress | `count_checkboxes()`, `render_progress_bar()` |
| `files.rs` | File constants and discovery | `find_existing_ralph_files()`, `find_archivable_files()`, `archive_base_dir()` |
| `templates.rs` | GitHub fetch with XDG cache | `get_all_templates()`, `fetch_template()` |
| `error.rs` | Unix-style errors, exit codes | `die()`, `exit` module |

### Key Patterns

**Subprocess execution** (`run.rs`): `spawn_claude()` pipes PROMPT.md to `claude -p` via stdin, spawns threads for real-time stdout/stderr streaming, captures output for magic string detection.

**Interview mode** (`main.rs`): Launches `claude` interactively with `--system-prompt` containing Ralph Loop context and `--allowedTools` restricted to: AskUserQuestion, Read, Glob, Grep, Write, Edit.

**Template caching** (`templates.rs`): Network-first strategy—fetch from GitHub, cache locally, fall back to cache on failure. Cache: `~/.cache/ralphctl/templates/` (Linux) or `~/Library/Caches/ralphctl/templates/` (macOS).

**Magic strings** (`run.rs`): Loop control signals in Claude output:
- `[[RALPH:CONTINUE]]` — Task completed, more tasks remain; loop continues automatically
- `[[RALPH:DONE]]` — All tasks complete; exit successfully
- `[[RALPH:BLOCKED:<reason>]]` — Cannot proceed; exit with code 3 for human intervention

Detection order: BLOCKED checked first, then CONTINUE/DONE (first match wins).

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
├── archive.rs
├── clean.rs
└── init.rs
```

## Ralph Workflow Files

| File | Purpose | Created By |
|------|---------|------------|
| `SPEC.md` | Project specification | init, interview |
| `IMPLEMENTATION_PLAN.md` | Task list with checkboxes | init, interview |
| `PROMPT.md` | Orchestration prompt piped to Claude | init |
| `ralph.log` | Iteration output log | run |
| `.ralphctl/archive/<timestamp>/` | Archived specs and plans | archive |

## CI/CD

GitHub Actions (`.github/workflows/ci.yml`):
- Multi-OS testing (Ubuntu, macOS)
- `cargo clippy -D warnings`
- `cargo fmt --check`
- All tests must pass

## Code Style

- Terse Unix-style errors: `error: claude not found in PATH`
- `anyhow::Result` for fallible functions
- Early returns over deep nesting
- No emojis in code or output
