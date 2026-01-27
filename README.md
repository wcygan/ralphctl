# ralphctl

A CLI tool for managing Ralph Loop workflows—autonomous development sessions driven by Claude.

## Installation

```bash
# From GitHub
cargo install --git https://github.com/wcygan/ralphctl

# From source
cargo install --path .
```

Requires the `claude` CLI to be installed and available in PATH.

## Usage

```bash
# Initialize a new ralph loop (fetches templates from GitHub)
ralphctl init

# Run the autonomous development loop
ralphctl run

# Check progress
ralphctl status

# Clean up ralph files
ralphctl clean
```

## Commands

| Command | Description |
|---------|-------------|
| `init` | Scaffold ralph loop files from templates |
| `run` | Execute the loop until done or blocked |
| `status` | Show progress bar with task completion stats |
| `clean` | Remove ralph loop files |

## How It Works

1. `ralphctl init` creates `SPEC.md`, `IMPLEMENTATION_PLAN.md`, and `PROMPT.md`
2. Fill in your project specification and task list
3. `ralphctl run` pipes the prompt to `claude -p` and streams output
4. Claude completes tasks one at a time, updating the plan
5. Loop continues until `[[RALPH:DONE]]` or `[[RALPH:BLOCKED:<reason>]]`

## License

MIT
