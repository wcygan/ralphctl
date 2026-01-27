# Project Specification: ralphctl

## Overview

`ralphctl` is a command-line tool for managing Ralph Loop workflows—autonomous development sessions driven by Claude. It provides utilities for initializing, running, monitoring, and cleaning up ralph loops, replacing the current bash-based `ralph.sh` orchestrator with a more robust, cross-platform Rust binary that can be easily distributed to other developers.

## User Experience

### Primary Workflows

1. **Initialize a new ralph loop**: User runs `ralphctl init` to scaffold template files (SPEC.md, IMPLEMENTATION_PLAN.md, PROMPT.md) fetched from GitHub. SPEC.md and IMPLEMENTATION_PLAN.md are bare section headers for the user to fill in; PROMPT.md contains the preconfigured ralph orchestration prompt.

2. **Run an existing loop**: User runs `ralphctl run` to execute the autonomous development loop. Picks up where it left off (resume semantics). Claude output streams through in real-time with iteration headers.

3. **Check progress**: User runs `ralphctl status` to see a Unicode progress bar with task completion statistics.

4. **Clean up artifacts**: User runs `ralphctl clean` to remove ralph-generated files with confirmation prompt (unless `--force`).

### Installation Experience

```bash
# macOS/Linux - Direct download
curl -fsSL https://github.com/wcygan/ralphctl/releases/latest/download/ralphctl-$(uname -s)-$(uname -m) -o ralphctl
chmod +x ralphctl
./ralphctl --help

# Cargo install (Rust users)
cargo install ralphctl
```

## Functional Requirements

- **FR1**: `ralphctl init` MUST fetch templates from GitHub (cache locally for offline), verify claude CLI exists, and generate SPEC.md (section headers only), IMPLEMENTATION_PLAN.md (section headers only), and PROMPT.md (preconfigured orchestration prompt). Refuse if files exist unless `--force`.

- **FR2**: `ralphctl run` MUST:
  - Validate PROMPT.md, SPEC.md, and IMPLEMENTATION_PLAN.md exist before starting
  - Read PROMPT.md verbatim and pipe to `claude -p` via stdin
  - Stream claude output in real-time (pass-through)
  - Print iteration header before each: `=== Iteration N starting ===`
  - Detect `[[RALPH:DONE]]` marker for completion
  - Detect `[[RALPH:BLOCKED:<reason>]]` marker, display reason, and exit immediately
  - If no marker after completion, prompt user for action
  - Resume from current state (no fresh-start; iteration count continues)
  - Respect `--max-iterations` (default: 50)
  - Log each iteration to ralph.log in append mode with structured sections

- **FR3**: `ralphctl status` MUST parse IMPLEMENTATION_PLAN.md, count all checkboxes (flat, no nesting weight), and display Unicode progress bar with stats: `[████████░░░░] 60% (12/20 tasks)`

- **FR4**: `ralphctl clean` MUST:
  - Remove SPEC.md, IMPLEMENTATION_PLAN.md, PROMPT.md, ralph.log
  - Show confirmation prompt `Delete N ralph files? [y/N]` (default No) unless `--force`
  - Succeed with message if no files found: `No ralph files found.`

- **FR5**: All commands MUST work on macOS (arm64, x86_64) and Linux (x86_64, arm64). Windows is explicitly out of scope.

- **FR6**: `ralphctl run` MUST log all iterations to ralph.log with structured sections:
  ```
  === ITERATION 5 ===
  Timestamp: 2025-01-26T10:30:00Z
  [full claude output]
  === END ===
  ```

- **FR7**: `ralphctl run` MUST handle Ctrl+C gracefully: forward signal to child process, wait for it to exit, then print summary (`Interrupted after N iterations. X/Y tasks complete.`) and exit.

- **FR8**: `ralphctl run --pause` MUST prompt `Ready for iteration N. Press Enter...` before each iteration starts.

## Success Criteria

- [ ] Single static binary with minimal runtime dependencies
- [ ] Cross-platform builds via GitHub Actions + cross-rs (macOS arm64/x86_64, Linux x86_64/arm64)
- [ ] `ralphctl init` fetches templates from GitHub, caches locally
- [ ] `ralphctl run` successfully completes a simple multi-task project autonomously
- [ ] `ralphctl status` displays accurate progress bar and completion percentage
- [ ] Startup time under 50ms (excluding network)

## Out of Scope

- **GUI or TUI**: This is a CLI tool only; no interactive terminal UI beyond prompts
- **Built-in LLM**: Requires external `claude` CLI; does not embed any AI
- **Project templates**: Does not scaffold new project structures, only ralph files
- **Remote execution**: No cloud/server mode; local execution only
- **Plugin system**: No extensibility mechanism in v1
- **Windows support**: Unix-only (macOS + Linux)
- **Shell completions**: Not included in v1
- **Binary size constraints**: No size limit; functionality over size
- **Homebrew formula**: Deferred to post-v1
- **Nix flake**: Deferred to post-v1

---

# Technical Architecture

## Technology Decisions

- **Language**: Rust
  - Rationale: Single static binary, excellent cross-compilation story, strong CLI ecosystem (clap, indicatif), memory safety without GC pauses
  - Trade-off: Longer compile times vs Go, steeper learning curve for contributors

- **CLI Framework**: `clap` (derive macros)
  - Rationale: Industry standard for Rust CLIs, excellent help generation

- **Async Runtime**: `tokio`
  - Rationale: Required for reqwest; entire CLI uses `#[tokio::main]`

- **HTTP Client**: `reqwest`
  - Rationale: Mature, reliable TLS handling, async-native

- **Markdown Parsing**: Simple regex
  - Rationale: Only need to parse checkbox syntax `- [ ]` / `- [x]`; full parser is overkill

- **Cross-compilation**: `cross-rs`
  - Rationale: Docker-based cross-compilation, handles C dependencies automatically

- **Cache Directory**: `dirs` crate
  - Rationale: XDG-compliant with platform-specific fallbacks

## Architectural Constraints

- **Network calls for templates only**: ralphctl fetches templates from GitHub during `init`; all other operations are local
- **Template caching**: Templates cached in XDG cache directory; try fetch latest, fall back to cached
- **Filesystem state**: All state stored in local files (SPEC.md, IMPLEMENTATION_PLAN.md, PROMPT.md, ralph.log)
- **Claude CLI dependency**: Assumes `claude` binary is in PATH; `init` and `run` error clearly if missing
- **UTF-8 assumed**: All file I/O assumes UTF-8 encoding

## Key Design Decisions

1. **Subprocess execution model**
   - Spawn `claude -p` as subprocess, pipe PROMPT.md via stdin, stream stdout/stderr in real-time
   - Rationale: Matches current ralph.sh behavior, allows claude CLI to handle its own auth/config
   - Trade-off: Dependent on claude CLI interface stability

2. **State file format**
   - Plain markdown files (same as current ralph workflow)
   - Rationale: Human-readable, editable, no migration needed from existing setups
   - Trade-off: Parsing markdown is less robust than structured formats

3. **Init workflow**
   - `ralphctl init` fetches templates from `https://raw.githubusercontent.com/wcygan/ralphctl/main/templates/`
   - Templates cached to XDG cache dir; used as fallback if network unavailable
   - Refuses to overwrite existing files without `--force`
   - Verifies `claude` CLI exists before creating files

4. **Configuration**
   - CLI flags only, no config file
   - Rationale: Simpler, all options visible in `--help`, no hidden state
   - Trade-off: Long commands if customizing heavily (but rare)

5. **Loop detection signals**
   - Primary: Magic strings `[[RALPH:DONE]]` and `[[RALPH:BLOCKED:<reason>]]` in claude output
   - Fallback: If no marker after iteration completes, prompt user for action
   - Rationale: Explicit signals are more reliable than parsing plan state

6. **Error messaging**
   - Terse Unix-style errors: `error: claude not found in PATH`
   - Rationale: Matches user preference for concise output

## Template Structure

Templates fetched from GitHub:

- `SPEC.md`: Section headers only (`# Overview`, `# Requirements`, etc.)
- `IMPLEMENTATION_PLAN.md`: Section headers only (`# Phase 1`, etc.)
- `PROMPT.md`: Full ralph orchestration prompt (preconfigured)

Template URL pattern: `https://raw.githubusercontent.com/wcygan/ralphctl/main/templates/{filename}`

## Dependencies

- `clap`: CLI argument parsing and help generation
- `anyhow`: Error handling with context
- `tokio`: Async runtime
- `reqwest`: HTTP client for template fetching
- `dirs`: XDG-compliant cache directory resolution
- `regex`: Parsing checkbox status from markdown
- `indicatif`: Progress bar rendering (optional, for status command)

## Non-Functional Requirements

- **Performance**: Startup < 50ms (excluding network), status parsing < 100ms for 1000-line plan
- **Reliability**: Graceful handling of interrupted loops, corrupt files, missing claude CLI, network failures
- **Portability**: Static linking where possible, no glibc version requirements on Linux

---

# Task Breakdown

## Phase 1: Project Setup

- [ ] **Task 1.1**: Initialize Cargo project with basic structure
  - Context: Fresh start
  - Acceptance: `cargo build` succeeds, produces `ralphctl` binary

- [ ] **Task 1.2**: Set up clap with subcommand structure (init, run, status, clean)
  - Context: Cargo project exists
  - Acceptance: `ralphctl --help` shows all subcommands, `ralphctl <cmd> --help` works

- [ ] **Task 1.3**: Add CI workflow for building and testing
  - Context: Basic project structure
  - Acceptance: GitHub Actions runs `cargo test` and `cargo clippy` on PRs and main branch

## Phase 2: Core Commands

- [ ] **Task 2.1**: Implement `ralphctl status` command
  - Context: Clap structure in place
  - Acceptance: Parses IMPLEMENTATION_PLAN.md, outputs Unicode progress bar with stats

- [ ] **Task 2.2**: Implement `ralphctl clean` command
  - Context: Clap structure in place
  - Acceptance: Removes ralph files with `[y/N]` confirmation, `--force` skips prompt, succeeds if no files

- [ ] **Task 2.3**: Implement `ralphctl init` command
  - Context: Clean command works
  - Acceptance: Fetches templates from GitHub, caches locally, refuses without `--force` if files exist, verifies claude CLI

- [ ] **Task 2.4**: Implement `ralphctl run` command (core loop)
  - Context: Status and init commands work
  - Acceptance: Pipes PROMPT.md to `claude -p`, streams output, detects magic strings, respects --max-iterations

## Phase 3: Robustness & Polish

- [ ] **Task 3.1**: Add logging to file during `run`
  - Context: Run command works
  - Acceptance: Each iteration logged with structured sections to ralph.log (append mode)

- [ ] **Task 3.2**: Add graceful Ctrl+C handling
  - Context: Run command works
  - Acceptance: Forwards signal to child, prints summary, exits cleanly

- [ ] **Task 3.3**: Add `--pause` flag for interactive confirmation between iterations
  - Context: Run command works
  - Acceptance: With `--pause`, prompts `Ready for iteration N. Press Enter...` before each iteration

- [ ] **Task 3.4**: Add fallback handling for missing magic strings
  - Context: Run command works
  - Acceptance: If no marker detected after iteration, prompts user for action

## Phase 4: Distribution

- [ ] **Task 4.1**: Set up cross-rs for cross-compilation
  - Context: All features complete
  - Acceptance: `cross build --target x86_64-unknown-linux-gnu` produces working binary

- [ ] **Task 4.2**: Add GitHub Actions release workflow
  - Context: Cross-compilation works
  - Acceptance: Git tag `v*` triggers builds for macOS (arm64, x86_64), Linux (x86_64, arm64)

## Phase 5: Documentation

- [ ] **Task 5.1**: Write README with installation, usage, examples
  - Context: All features complete
  - Acceptance: README covers all commands with examples

- [ ] **Task 5.2**: Add `--help` text polish and examples in clap
  - Context: All commands implemented
  - Acceptance: Help text is clear with usage examples

- [ ] **Task 5.3**: Create CHANGELOG.md for v0.1.0
  - Context: Ready to release
  - Acceptance: Documents all features in initial release

- [ ] **Task 5.4**: Create templates directory with SPEC.md, IMPLEMENTATION_PLAN.md, PROMPT.md
  - Context: Ready to release
  - Acceptance: Templates exist in repo at templates/ directory

## Testing Strategy

- **Unit tests**: Markdown checkbox parsing, template caching logic, argument validation
- **Integration tests**: Full command execution with fixture files, mock HTTP responses
- **Platform tests**: CI matrix covers macOS and Linux
- **E2E tests**: Deferred to post-v1 (manual testing for now)

---

# CLI Reference

## Commands

### `ralphctl init`

Scaffold ralph loop files from templates.

```
ralphctl init [--force]
```

**Flags:**
- `--force`: Overwrite existing files without prompting

**Behavior:**
1. Verify `claude` CLI is in PATH (error if missing)
2. Check if PROMPT.md, SPEC.md, or IMPLEMENTATION_PLAN.md exist
3. If files exist and no `--force`: error with message
4. Fetch templates from GitHub (cache for offline use)
5. Write files to current directory

**Exit codes:**
- 0: Success
- 1: Error (claude missing, files exist, network failed with no cache)

### `ralphctl run`

Execute the ralph loop until done or blocked.

```
ralphctl run [--max-iterations N] [--pause]
```

**Flags:**
- `--max-iterations N`: Maximum iterations before stopping (default: 50)
- `--pause`: Prompt for confirmation before each iteration

**Behavior:**
1. Validate PROMPT.md, SPEC.md, IMPLEMENTATION_PLAN.md exist
2. For each iteration:
   - Print `=== Iteration N starting ===`
   - If `--pause`: prompt `Ready for iteration N. Press Enter...`
   - Pipe PROMPT.md to `claude -p`, stream output
   - Log iteration to ralph.log
   - Check for `[[RALPH:DONE]]` → exit success
   - Check for `[[RALPH:BLOCKED:<reason>]]` → print reason, exit
   - If no marker, prompt user for action
3. If max iterations reached, print summary and exit

**Exit codes:**
- 0: Completed (RALPH:DONE detected)
- 1: Blocked (RALPH:BLOCKED detected)
- 2: Max iterations reached
- 130: Interrupted (Ctrl+C)

### `ralphctl status`

Show ralph loop progress.

```
ralphctl status
```

**Output:**
```
[████████░░░░] 60% (12/20 tasks)
```

**Exit codes:**
- 0: Success
- 1: IMPLEMENTATION_PLAN.md not found

### `ralphctl clean`

Remove ralph loop files.

```
ralphctl clean [--force]
```

**Flags:**
- `--force`: Skip confirmation prompt

**Behavior:**
1. Find ralph files: SPEC.md, IMPLEMENTATION_PLAN.md, PROMPT.md, ralph.log
2. If no files found: print `No ralph files found.` and exit 0
3. If files found and no `--force`: prompt `Delete N ralph files? [y/N]`
4. On confirmation or `--force`: delete files

**Exit codes:**
- 0: Success (or no files to clean)
- 1: User declined confirmation

---

# Specification Evolution

## Version History

- v2.0 (2025-01-26): Major revision after interview - template fetch, streaming output, magic strings, Unix-only
- v1.0 (2025-01-26): Initial specification

## Open Questions (Resolved)

- ~~Should `ralphctl init` embed the full interview prompt or fetch it from a URL?~~ → Fetch from GitHub, cache locally
- ~~Should there be a `ralphctl resume` command distinct from `run`?~~ → No, `run` always resumes
- ~~What's the right default for `--max-iterations`?~~ → 50 (matches ralph.sh)

## Future Considerations

- **Homebrew formula**: Set up wcygan/homebrew-tap for easier installation
- **Nix flake**: Add flake.nix for Nix users
- **Shell completions**: Add `ralphctl completions` subcommand
- **TUI mode**: Interactive terminal UI for monitoring long-running loops
- **Config file**: `.ralphctl.toml` for project-specific defaults
- **Template system**: Different prompt templates for different project types
