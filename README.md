# ralphctl

A CLI tool for managing Ralph Loop workflows—autonomous development sessions driven by Claude.

## Installation

Install as a binary compiled from source:

```bash
cargo install --git https://github.com/wcygan/ralphctl
```

### Prerequisites

Requires the `claude` CLI to be installed and available in PATH. You must be authenticated with Claude.

## Quickstart

These 3 self-contained commands get you started:

```bash
ralphctl init
ralphctl interview
ralphctl run
```

They will setup the necessary structure, create context in the correct format, and [run the loop](https://github.com/wcygan/ralphctl/blob/368a650db9ab6c9385cec19ac94074744f0669ec/src/main.rs#L372).

## Workflow

```
init → interview → run → verify → archive (or clean)
```

1. **`ralphctl init`** — Scaffold ralph loop files from templates
2. **`ralphctl interview`** — AI-guided interview to create SPEC.md and IMPLEMENTATION_PLAN.md
3. **`ralphctl run`** — Execute the autonomous development loop
4. **Verify** — Manually review the completed work
5. **`ralphctl archive`** — Save spec/plan to `.ralphctl/archive/` and reset for next loop
6. **`ralphctl clean`** — Remove ralph loop files when done

## Quick Start

```bash
# 1. Initialize a new ralph loop
ralphctl init

# 2. Interview to define your project (interactive)
ralphctl interview

# 3. Run the autonomous development loop
ralphctl run

# 4. Check progress at any time
ralphctl status

# 5a. Archive completed work and start fresh
ralphctl archive

# 5b. Or clean up when completely done
ralphctl clean
```

## Commands

### `ralphctl init`

Scaffold ralph loop files from templates.

```bash
ralphctl init [--force]
```

| Flag | Description |
|------|-------------|
| `--force` | Overwrite existing files without prompting |

Creates `SPEC.md`, `IMPLEMENTATION_PLAN.md`, and `PROMPT.md` in the current directory. Templates are fetched from GitHub and cached locally for offline use.

### `ralphctl interview`

Interactive AI-guided interview to create project spec and implementation plan.

```bash
ralphctl interview [--model <MODEL>]
```

| Flag | Description |
|------|-------------|
| `--model` | Claude model to use (default: sonnet) |

Launches an interactive Claude session that asks questions about your project and generates a detailed SPEC.md and IMPLEMENTATION_PLAN.md.

### `ralphctl run`

Execute the ralph loop until done or blocked.

```bash
ralphctl run [--max-iterations N] [--pause] [--model <MODEL>]
```

| Flag | Description |
|------|-------------|
| `--max-iterations` | Maximum iterations before stopping (default: 50) |
| `--pause` | Prompt for confirmation before each iteration |
| `--model` | Claude model to use (default: sonnet) |

The loop reads PROMPT.md and pipes it to `claude -p`, streaming output in real-time. Each iteration is logged to `ralph.log`.

**Exit codes:**
- `0` — Completed (`[[RALPH:DONE]]` detected)
- `1` — General error
- `2` — Max iterations reached
- `3` — Blocked (`[[RALPH:BLOCKED]]` detected)
- `130` — Interrupted (Ctrl+C)

### `ralphctl status`

Show ralph loop progress.

```bash
ralphctl status
```

Parses IMPLEMENTATION_PLAN.md and displays a progress bar:

```
[████████░░░░] 60% (12/20 tasks)
```

### `ralphctl archive`

Save spec and plan to timestamped archive, reset for next loop.

```bash
ralphctl archive [--force]
```

| Flag | Description |
|------|-------------|
| `--force` | Skip confirmation prompt |

Archives SPEC.md and IMPLEMENTATION_PLAN.md to `.ralphctl/archive/<timestamp>/`, then replaces them with blank templates.

### `ralphctl clean`

Remove ralph loop files.

```bash
ralphctl clean [--force]
```

| Flag | Description |
|------|-------------|
| `--force` | Skip confirmation prompt |

Removes SPEC.md, IMPLEMENTATION_PLAN.md, PROMPT.md, and ralph.log.

### `ralphctl update`

Install the latest version of ralphctl from GitHub.

```bash
ralphctl update
```

Runs `cargo install --git https://github.com/wcygan/ralphctl` to fetch and compile the latest release.

## How It Works

The Ralph Loop is an autonomous development workflow:

1. `ralphctl init` creates `SPEC.md`, `IMPLEMENTATION_PLAN.md`, and `PROMPT.md`
2. `ralphctl interview` guides you through defining your project specification and task list
3. `ralphctl run` pipes the prompt to `claude -p` and streams output
4. Claude reads the spec, finds the next unchecked task, implements it, and marks it complete
5. Loop repeats with fresh context each iteration (avoiding context rot)
6. Loop exits when Claude outputs `[[RALPH:DONE]]` or `[[RALPH:BLOCKED:<reason>]]`

### Why Fresh Context?

Each iteration starts with clean context. This eliminates "context rot"—the degradation of AI performance as conversation history accumulates with stale information and abandoned approaches. Local files (SPEC.md, IMPLEMENTATION_PLAN.md) serve as persistent memory across iterations.

### Magic Strings

The loop detects these signals in Claude's output:

- `[[RALPH:CONTINUE]]` — Task completed, more tasks remain; loop continues automatically
- `[[RALPH:DONE]]` — All tasks complete, exit successfully
- `[[RALPH:BLOCKED:<reason>]]` — Cannot proceed, requires human intervention

## Ralph Loop Files

| File | Purpose |
|------|---------|
| `SPEC.md` | Project specification and requirements |
| `IMPLEMENTATION_PLAN.md` | Task list with checkboxes |
| `PROMPT.md` | Orchestration prompt piped to Claude |
| `ralph.log` | Iteration output log |
| `.ralphctl/archive/` | Archived specs and plans |

## License

MIT
