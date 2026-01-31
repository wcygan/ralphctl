---
sidebar_position: 2
title: How ralphctl Implements It
description: How ralphctl's design decisions embody the philosophical principles of the Ralph Loop.
---

# How ralphctl Implements It

This page maps the philosophical principles from [The Philosophy](./the-philosophy.md) to specific design decisions in ralphctl's codebase. Each section shows how an abstract idea becomes working code.

## Context Engineering in Practice

### Fresh Subprocess Invocations

Each iteration of the ralph loop spawns a new `claude -p` subprocess. The `spawn_claude()` function in `run.rs` creates a fresh `Command`, pipes PROMPT.md via stdin, and captures the output:

```rust
let mut cmd = Command::new("claude");
cmd.arg("-p")
    .arg("--dangerously-skip-permissions")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
```

The `-p` flag runs Claude in non-interactive (pipe) mode -- it reads from stdin, processes the input, writes to stdout, and exits. No conversation state persists between invocations. This is the architectural foundation for fresh context: each iteration gets a clean context window with zero history from previous iterations.

The prompt content is written to stdin and then the pipe is closed, signaling EOF to Claude:

```rust
if let Some(mut stdin) = child.stdin.take() {
    stdin.write_all(prompt.as_bytes())?;
    // stdin is dropped here, closing the pipe
}
```

### State Lives in Files

ralphctl uses three file-based state mechanisms, none of which carry conversation history:

- **SPEC.md** holds the project specification -- requirements, architecture, and scope. The agent reads this every iteration to understand what it's building.
- **IMPLEMENTATION_PLAN.md** tracks progress through markdown checkboxes (`- [ ]` and `- [x]`). The agent reads the plan to find the next unchecked task, and writes back to mark tasks complete.
- **PROMPT.md** contains the orchestration instructions that tell the agent how to behave -- read the state files, implement one task, run tests, commit, and signal.

This is file-based memory, not conversation-based memory. The agent doesn't need to recall what it did three iterations ago. It reads the current state of the project from disk and acts on what it finds. If 12 of 20 checkboxes are checked, the agent picks task 13 -- regardless of whether it was the agent that completed the first 12 or a human developer.

The `validate_required_files()` function in `run.rs` enforces that all three state files exist before the loop starts, preventing iterations from running against incomplete state.

## Software as Clay in Practice

### The Interview-Run-Archive Lifecycle

ralphctl's command set maps directly to the clay metaphor:

1. **`ralphctl interview`** shapes the initial clay. It launches Claude in interactive mode with a system prompt that guides the user through defining their project. The output is a SPEC.md and IMPLEMENTATION_PLAN.md -- the mold for the clay.

2. **`ralphctl run`** is the wheel spinning. Each iteration refines the codebase by implementing one task. If the agent makes a mistake, the next iteration can correct it. The loop continues until all tasks are complete or the agent hits a blocker.

3. **`ralphctl archive`** clears the wheel. It copies the spec and plan to a timestamped directory under `.ralphctl/archive/`, then resets the originals to blank templates. The workspace is clean for the next project.

The archive function generates filesystem-safe timestamps and handles both forward mode (SPEC.md, IMPLEMENTATION_PLAN.md) and reverse mode (QUESTION.md, INVESTIGATION.md, FINDINGS.md) files. Files that have reset templates get blanked; files without templates (like FINDINGS.md) are deleted entirely.

This lifecycle means no project state is permanent. Everything is shaped, archived, and replaced. The codebase itself is the lasting artifact -- the files that drive the loop are disposable scaffolding.

## Single-Task Loops in Practice

### One Checkbox Per Iteration

PROMPT.md instructs the agent to find the first unchecked task (`- [ ]`), implement it completely, mark it as checked (`- [x]`), and then emit a signal. The orchestrator in `main.rs` enforces this contract by checking for control signals after each iteration.

The agent can't skip ahead or work on multiple tasks because the prompt explicitly forbids it -- and the signal mechanism creates a natural stopping point. After emitting `[[RALPH:CONTINUE]]`, the agent's process exits. The next iteration starts fresh with no memory of the previous one.

### Magic String Control Flow

Loop control uses three signal patterns, detected by scanning the agent's stdout line by line:

| Signal | Meaning | Exit behavior |
|--------|---------|---------------|
| `[[RALPH:CONTINUE]]` | Task done, more remain | Start next iteration |
| `[[RALPH:DONE]]` | All tasks complete | Exit with code 0 |
| `[[RALPH:BLOCKED:<reason>]]` | Cannot proceed | Exit with code 3 |

Detection is strict: signals must appear alone on a line (with optional whitespace). The `detect_signal()` function trims each line and compares against the exact marker strings. This prevents false positives when the agent discusses or quotes the signals in its output -- a line like `` The test covers `[[RALPH:DONE]]` detection `` won't trigger termination because the backticks make it not an exact match.

Detection order matters too. `detect_blocked_signal()` is checked first in the main loop, giving BLOCKED priority over CONTINUE or DONE. This ensures the loop stops for human intervention even if the agent also emits a completion signal.

## Additional Design Decisions

Beyond the direct philosophical mappings, several implementation choices reinforce the Ralph Loop's principles:

### XDG-Compliant Template Caching

The `templates.rs` module implements a network-first caching strategy for template files. Templates are fetched from GitHub on each `init`, cached locally in XDG-compliant directories (`~/.cache/ralphctl/templates/` on Linux, `~/Library/Caches/ralphctl/templates/` on macOS), and served from cache when the network is unavailable.

This supports the clay metaphor -- templates are cheap and replaceable. You can always get a fresh one from the network, but the cache ensures you're never blocked by a network outage.

### Strict Signal Detection

Signal markers must appear alone on their line. The `detect_signal()` function iterates over `output.lines()`, trims whitespace, and compares against exact marker strings. This is deliberately strict to prevent the loop from terminating when the agent merely mentions a signal in its explanatory text.

```rust
for line in output.lines() {
    let trimmed = line.trim();
    if trimmed == RALPH_DONE_MARKER {
        return LoopSignal::Done;
    }
}
```

### Unix-Style Exit Codes

ralphctl uses semantic exit codes that compose with standard Unix tooling:

| Code | Meaning |
|------|---------|
| 0 | Success (all tasks complete) |
| 1 | General error |
| 2 | Max iterations reached |
| 3 | Blocked (needs human intervention) |
| 130 | Interrupted (Ctrl+C) |

These are defined in `error.rs` and allow shell scripts, CI pipelines, and other tools to branch on the outcome of a ralph loop run. For example, a CI job could retry on exit code 2 but alert on exit code 3.

### Graceful Ctrl+C Handling

When a user presses Ctrl+C during a loop, ralphctl propagates the signal to the child Claude process via SIGTERM rather than killing it immediately. The `spawn_claude()` function sets up an interrupt flag checked by a polling thread, which sends SIGTERM to the child process group when triggered. This ensures the child process can clean up (e.g., finish writing files) before the loop exits with code 130.

The interrupt summary also reads IMPLEMENTATION_PLAN.md to report how many tasks were completed before interruption, giving the user immediate visibility into progress.
