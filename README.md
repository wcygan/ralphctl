# ralphctl

A CLI tool for managing Ralph Loop workflows—autonomous development sessions driven by Claude.

## Installation

```bash
# From GitHub
cargo install --git https://github.com/wcygan/ralphctl
```

Requires the `claude` CLI to be installed and available in PATH.

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

| Command | Description |
|---------|-------------|
| `init` | Scaffold ralph loop files from templates |
| `interview` | Interactive AI interview to create spec and plan |
| `run` | Execute the loop until done or blocked |
| `status` | Show progress bar with task completion stats |
| `archive` | Save spec/plan to timestamped archive, reset for next loop |
| `clean` | Remove ralph loop files |

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

## License

MIT
