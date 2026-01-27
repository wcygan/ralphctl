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

# Reverse Mode Specification

## Overview

Reverse Mode is a new command for `ralphctl` that enables autonomous investigation workflows. While Forward Mode (`ralphctl run`) builds software by completing tasks, Reverse Mode (`ralphctl reverse`) analyzes codebases to answer questions—diagnosing bugs, understanding legacy code, or mapping dependencies before refactoring. It operates read-only by design and produces structured findings documents.

## Goals

- Enable autonomous investigation of codebases without code modification
- Provide structured hypothesis-driven investigation with checkboxes for tracking
- Produce actionable findings documents (INVESTIGATION.md and FINDINGS.md)
- Support the same iteration model as forward mode (fresh context each iteration)
- Integrate cleanly with existing ralphctl commands (clean, archive)

## Non-Goals

- Hard enforcement of read-only behavior (trust the prompt)
- GitHub issue integration (user can copy-paste)
- Separate archive/clean commands for reverse files

## CLI Interface

```
ralphctl reverse [OPTIONS] [QUESTION]

Arguments:
  [QUESTION]  The investigation question (reads from QUESTION.md if omitted)

Options:
      --max-iterations <N>  Maximum iterations before stopping [default: 100]
      --pause               Prompt for confirmation before each iteration
      --model <MODEL>       Claude model to use (e.g., 'sonnet', 'opus')
  -h, --help                Print help
```

### Usage Examples

```bash
# Provide question directly
ralphctl reverse "Why does the authentication flow fail for OAuth users?"

# Use existing QUESTION.md
ralphctl reverse

# With options
ralphctl reverse --model opus --max-iterations 50 "Why is the cache invalidation slow?"
ralphctl reverse --pause "How does the payment processing work?"
```

## File Ecosystem

Reverse Mode uses its own files, independent from Forward Mode:

| File | Created By | Purpose |
|------|------------|---------|
| `QUESTION.md` | User or `ralphctl reverse` | The investigation question |
| `INVESTIGATION.md` | Claude agent | Running log of hypotheses with checkboxes |
| `FINDINGS.md` | Claude agent | Final synthesized report |
| `REVERSE_PROMPT.md` | `ralphctl reverse` (fetched) | Instructions for investigation loop |

### QUESTION.md Structure

Created automatically when user runs `ralphctl reverse "question"` or as a template when no argument provided:

```markdown
# Investigation Question

<user's question here>

## Context (Optional)

<any additional context the user wants to provide>
```

Minimal template (when created without argument):

```markdown
# Investigation Question

Describe what you want to investigate...
```

### INVESTIGATION.md Structure

Created and maintained by the Claude agent during investigation:

```markdown
# Investigation Log

**Question:** <copied from QUESTION.md>
**Started:** <timestamp>
**Status:** In Progress | Answered | Inconclusive

## Hypothesis 1: <title>
- [ ] Check <thing>
- [x] Examined <thing> — <finding>
- **Result:** Ruled Out | Confirmed | Partially Confirmed

## Hypothesis 2: <title>
- [ ] Investigate <aspect>
- [x] Found <evidence>
- **Result:** Ruled Out

## Dead Ends
- <approach that didn't work and why>

## Key Findings
- <important discoveries along the way>
```

### FINDINGS.md Structure

Created by the Claude agent when investigation concludes:

```markdown
# Investigation Findings

**Question:** <original question>
**Status:** Answered | Inconclusive
**Date:** <timestamp>

## Summary

<1-2 paragraph answer to the question>

## Evidence

<supporting details, file references, code snippets>

## Recommendations

<suggested next steps or actions>

## Investigation Path

<brief summary of hypotheses explored>
```

## Signal Protocol

Reverse Mode uses these signals:

| Signal | Meaning | Exit Code |
|--------|---------|-----------|
| `[[RALPH:CONTINUE]]` | Still investigating, more hypotheses to explore | (loop continues) |
| `[[RALPH:FOUND:<summary>]]` | Question answered, FINDINGS.md written | 0 |
| `[[RALPH:INCONCLUSIVE:<why>]]` | Cannot determine answer, FINDINGS.md written | 4 |
| `[[RALPH:BLOCKED:<reason>]]` | Cannot proceed (same as forward mode) | 3 |

Detection priority: BLOCKED → FOUND → INCONCLUSIVE → CONTINUE (first match wins)

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success (FOUND signal detected) |
| 1 | General error |
| 2 | Max iterations reached (default: 100) |
| 3 | Blocked (BLOCKED signal detected) |
| 4 | Inconclusive (INCONCLUSIVE signal detected) |
| 130 | Interrupted (Ctrl+C) |

## Iteration Flow

1. **User invokes**: `ralphctl reverse "Why does auth fail?"`
2. **Question setup**:
   - If argument provided: write to QUESTION.md
   - If no argument and QUESTION.md exists: use existing file
   - If no argument and no QUESTION.md: create template, print instructions, exit 1
3. **Template fetch**: Fetch/cache REVERSE_PROMPT.md from GitHub (like PROMPT.md)
4. **Loop iteration**:
   a. Print `=== Iteration N starting ===`
   b. If `--pause`: prompt for confirmation
   c. Pipe REVERSE_PROMPT.md to `claude -p --dangerously-skip-permissions`
   d. Claude reads QUESTION.md and INVESTIGATION.md (if exists)
   e. Claude explores codebase, updates INVESTIGATION.md with hypothesis status
   f. Claude outputs signal or continues investigating
5. **Termination**:
   - On FOUND: Claude has written FINDINGS.md, exit 0
   - On INCONCLUSIVE: Claude has written FINDINGS.md, exit 4
   - On BLOCKED: Print reason, exit 3
   - On max iterations: Print warning, exit 2
   - On Ctrl+C: Print summary, exit 130

## Integration with Existing Commands

### `ralphctl clean`

Extended to handle reverse files:

```rust
// All ralph files (forward + reverse)
const ALL_RALPH_FILES: &[&str] = &[
    // Forward mode
    "SPEC.md", "IMPLEMENTATION_PLAN.md", "PROMPT.md", "ralph.log",
    // Reverse mode
    "QUESTION.md", "INVESTIGATION.md", "FINDINGS.md", "REVERSE_PROMPT.md",
];
```

Behavior: `ralphctl clean` removes all ralph files (both modes) with same confirmation UX.

### `ralphctl archive`

Extended to archive reverse files:

- Archive: QUESTION.md, INVESTIGATION.md, FINDINGS.md (not REVERSE_PROMPT.md - it's a template)
- Same timestamped directory structure: `.ralphctl/archive/<timestamp>/`
- Reset: QUESTION.md and INVESTIGATION.md reset to blank (FINDINGS.md deleted)

## Implementation Architecture

### New Module: `reverse.rs`

```rust
//! Reverse mode implementation for ralphctl.
//!
//! Provides investigation loop logic distinct from forward mode.

use crate::{error, files};
use anyhow::Result;
use std::path::Path;

/// Reverse mode signal types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReverseSignal {
    Continue,             // [[RALPH:CONTINUE]] - still investigating
    Found(String),        // [[RALPH:FOUND:<summary>]]
    Inconclusive(String), // [[RALPH:INCONCLUSIVE:<why>]]
    Blocked(String),      // [[RALPH:BLOCKED:<reason>]]
    NoSignal,
}

/// Validate reverse mode files exist.
pub fn validate_reverse_files() -> Result<()>;

/// Read the investigation question.
pub fn read_question() -> Result<String>;

/// Create minimal QUESTION.md template.
pub fn create_question_template() -> Result<()>;

/// Detect reverse mode signals in output.
/// Priority: BLOCKED → FOUND → INCONCLUSIVE → CONTINUE
pub fn detect_reverse_signal(output: &str) -> ReverseSignal;

/// Signal markers
pub const RALPH_FOUND_PREFIX: &str = "[[RALPH:FOUND:";
pub const RALPH_INCONCLUSIVE_PREFIX: &str = "[[RALPH:INCONCLUSIVE:";
// Note: CONTINUE and BLOCKED markers are shared with forward mode (run.rs)
```

### Modified: `files.rs`

```rust
// Reverse mode files
pub const QUESTION_FILE: &str = "QUESTION.md";
pub const INVESTIGATION_FILE: &str = "INVESTIGATION.md";
pub const FINDINGS_FILE: &str = "FINDINGS.md";
pub const REVERSE_PROMPT_FILE: &str = "REVERSE_PROMPT.md";

// Combined file lists
pub const REVERSE_FILES: &[&str] = &[
    QUESTION_FILE,
    INVESTIGATION_FILE,
    FINDINGS_FILE,
    REVERSE_PROMPT_FILE,
];

pub fn find_existing_reverse_files(dir: &Path) -> Vec<PathBuf>;
pub fn find_archivable_reverse_files(dir: &Path) -> Vec<PathBuf>;
```

### Modified: `error.rs`

```rust
pub mod exit {
    pub const SUCCESS: i32 = 0;
    pub const ERROR: i32 = 1;
    pub const MAX_ITERATIONS: i32 = 2;
    pub const BLOCKED: i32 = 3;
    pub const INCONCLUSIVE: i32 = 4;  // NEW
    pub const INTERRUPTED: i32 = 130;
}
```

### Modified: `main.rs`

```rust
#[derive(Subcommand)]
enum Command {
    // ... existing commands ...

    /// Investigate a codebase to answer a question
    #[command(
        long_about = "Run an autonomous investigation loop to answer a question about the codebase.\n\n\
                      Unlike 'run' which builds software, 'reverse' analyzes code to answer questions—\n\
                      diagnosing bugs, understanding systems, or mapping dependencies before changes.",
        after_help = "EXAMPLES:\n  \
                      ralphctl reverse \"Why does the cache fail?\"\n  \
                      ralphctl reverse --model opus \"How does auth work?\"\n  \
                      ralphctl reverse --pause\n\n\
                      EXIT CODES:\n  \
                      0   Found (question answered)\n  \
                      1   Error\n  \
                      2   Max iterations reached\n  \
                      3   Blocked\n  \
                      4   Inconclusive\n  \
                      130 Interrupted"
    )]
    Reverse {
        /// The investigation question (reads from QUESTION.md if omitted)
        question: Option<String>,

        /// Maximum iterations before stopping
        #[arg(long, default_value = "100", value_name = "N")]
        max_iterations: u32,

        /// Prompt for confirmation before each iteration
        #[arg(long)]
        pause: bool,

        /// Claude model to use (e.g., 'sonnet', 'opus')
        #[arg(long, value_name = "MODEL")]
        model: Option<String>,
    },
}
```

### New Template: `REVERSE_PROMPT.md`

```markdown
# Ralph Reverse Mode Prompt

You are operating in an autonomous investigation loop. Your job is to answer a question about this codebase.

## Context Files

- `QUESTION.md` - The investigation question (read first)
- `INVESTIGATION.md` - Your running investigation log (read and update)

## Your Mission (Single Iteration)

### Step 1: Orient

1. Read `QUESTION.md` to understand what you're investigating
2. Read `INVESTIGATION.md` (if it exists) to see what you've already tried
3. Identify the next hypothesis to explore or the next check to perform

### Step 2: Investigate

1. Explore the codebase to gather evidence
2. Use Glob, Grep, and Read to examine relevant files
3. Follow the trail of evidence where it leads
4. Document your findings in INVESTIGATION.md

### Step 3: Update State

Update `INVESTIGATION.md` with:
- New hypotheses as `## Hypothesis N: <title>` with checkbox items
- Mark checked items as `- [x]` with findings
- Add dead ends and key findings sections as needed

### Step 4: Signal Completion

After your investigation work, output exactly one of these signals on its own line:

**Question answered (you have a confident answer):**
```
[[RALPH:FOUND:<brief summary of answer>]]
```
Before outputting this signal, you MUST write FINDINGS.md with your complete answer.

**Cannot determine answer (exhausted reasonable approaches):**
```
[[RALPH:INCONCLUSIVE:<why you can't determine the answer>]]
```
Before outputting this signal, you MUST write FINDINGS.md documenting what you tried and why it's inconclusive.

**Cannot proceed due to blocker:**
```
[[RALPH:BLOCKED:<reason>]]
```

---

## Rules

1. **Read-only intent** - Do not modify application code; only update INVESTIGATION.md and FINDINGS.md
2. **One hypothesis per iteration** - Explore one avenue, document findings, then signal
3. **Always document** - Update INVESTIGATION.md before signaling
4. **Write findings when done** - FINDINGS.md must exist before FOUND or INCONCLUSIVE
5. **Be thorough but focused** - Follow evidence but don't go on tangents
6. **Cite your sources** - Reference specific files and line numbers in findings

## INVESTIGATION.md Format

```markdown
# Investigation Log

**Question:** <from QUESTION.md>
**Started:** <timestamp>
**Status:** In Progress

## Hypothesis 1: <descriptive title>
- [ ] Check <specific thing>
- [x] Examined <thing> — <what you found>
- **Result:** Ruled Out | Confirmed | Partially Confirmed

## Dead Ends
- <approach that didn't pan out>

## Key Findings
- <important discoveries>
```

## FINDINGS.md Format

```markdown
# Investigation Findings

**Question:** <original question>
**Status:** Answered | Inconclusive
**Date:** <timestamp>

## Summary
<1-2 paragraph answer>

## Evidence
<file references, code snippets, proof>

## Recommendations
<suggested next steps>

## Investigation Path
<summary of what was explored>
```

---

**Begin by reading QUESTION.md, then start or continue your investigation.**
```

## Acceptance Criteria

1. **CLI Parsing**: `ralphctl reverse "question"` creates QUESTION.md and starts investigation
2. **No-arg behavior**: `ralphctl reverse` without args uses QUESTION.md or creates template
3. **Template creation**: Missing QUESTION.md triggers template creation with instructions
4. **Template fetching**: REVERSE_PROMPT.md fetched from GitHub and cached
5. **Signal detection**: FOUND, INCONCLUSIVE, BLOCKED signals detected correctly
6. **Exit codes**: Correct exit code for each termination condition
7. **Iteration loop**: Same subprocess spawning and streaming as forward mode
8. **Logging**: Iterations logged to ralph.log
9. **Pause mode**: --pause flag works identically to forward mode
10. **Model flag**: --model flag passed to claude subprocess
11. **Clean integration**: `ralphctl clean` removes reverse files
12. **Archive integration**: `ralphctl archive` archives reverse files
13. **Help text**: `ralphctl reverse --help` displays comprehensive usage

## Testing Strategy

- **Unit tests**: Signal detection for FOUND/INCONCLUSIVE/BLOCKED
- **Unit tests**: Question file reading and template creation
- **Integration tests**: Full reverse command with mock claude
- **Integration tests**: Clean/archive with reverse files present

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

### `ralphctl reverse`

Investigate a codebase to answer a question.

```
ralphctl reverse [OPTIONS] [QUESTION]
```

**Arguments:**
- `QUESTION`: The investigation question (optional; reads from QUESTION.md if omitted)

**Flags:**
- `--max-iterations N`: Maximum iterations before stopping (default: 100)
- `--pause`: Prompt for confirmation before each iteration
- `--model MODEL`: Claude model to use

**Behavior:**
1. If QUESTION provided: write to QUESTION.md
2. If no QUESTION and QUESTION.md missing: create template, print instructions, exit
3. Fetch/cache REVERSE_PROMPT.md
4. For each iteration:
   - Print `=== Iteration N starting ===`
   - If `--pause`: prompt for confirmation
   - Pipe REVERSE_PROMPT.md to `claude -p`, stream output
   - Log iteration to ralph.log
   - Check for signals (BLOCKED → FOUND → INCONCLUSIVE)
   - If no signal, prompt user for action
5. On termination signal or max iterations, exit with appropriate code

**Exit codes:**
- 0: Found (question answered)
- 1: Error
- 2: Max iterations reached
- 3: Blocked
- 4: Inconclusive
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
1. Find all ralph files (forward mode + reverse mode)
2. If no files found: print `No ralph files found.` and exit 0
3. If files found and no `--force`: prompt `Delete N ralph files? [y/N]`
4. On confirmation or `--force`: delete files

**Exit codes:**
- 0: Success (or no files to clean)
- 1: User declined confirmation

### `ralphctl archive`

Archive ralph files and reset.

```
ralphctl archive [--force]
```

**Flags:**
- `--force`: Skip confirmation prompt

**Behavior:**
1. Find archivable files (SPEC.md, IMPLEMENTATION_PLAN.md, QUESTION.md, INVESTIGATION.md, FINDINGS.md)
2. Copy to timestamped directory: `.ralphctl/archive/<timestamp>/`
3. Reset files to blank templates (delete FINDINGS.md)

---

# Specification Evolution

## Version History

- v3.0 (2025-01-27): Added Reverse Mode specification
- v2.0 (2025-01-26): Major revision after interview - template fetch, streaming output, magic strings, Unix-only
- v1.0 (2025-01-26): Initial specification

## Open Questions (Resolved)

- ~~Should `ralphctl init` embed the full interview prompt or fetch it from a URL?~~ → Fetch from GitHub, cache locally
- ~~Should there be a `ralphctl resume` command distinct from `run`?~~ → No, `run` always resumes
- ~~What's the right default for `--max-iterations`?~~ → 50 (matches ralph.sh)
- ~~Should reverse mode have its own log file?~~ → No, share ralph.log

## Future Considerations

- **Homebrew formula**: Set up wcygan/homebrew-tap for easier installation
- **Nix flake**: Add flake.nix for Nix users
- **Shell completions**: Add `ralphctl completions` subcommand
- **TUI mode**: Interactive terminal UI for monitoring long-running loops
- **Config file**: `.ralphctl.toml` for project-specific defaults
- **Template system**: Different prompt templates for different project types
- **Learning mode**: Educational verbosity for understanding how the loop works
- **Prompt tuning**: Guided prompt adjustment when investigations fail repeatedly
