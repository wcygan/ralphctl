mod cli;
mod error;
mod files;
mod parser;
mod run;
mod templates;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// Files that init creates (excludes ralph.log which is only created by run)
const INIT_FILES: &[&str] = &[
    files::SPEC_FILE,
    files::IMPLEMENTATION_PLAN_FILE,
    files::PROMPT_FILE,
];

#[derive(Parser)]
#[command(name = "ralphctl")]
#[command(version, about = "Manage Ralph Loop workflows")]
#[command(after_help = "Workflow: init → interview → run → verify → clean")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize ralph loop files from templates
    Init {
        /// Overwrite existing files without prompting
        #[arg(long)]
        force: bool,
    },

    /// Interactive interview to create SPEC.md and IMPLEMENTATION_PLAN.md
    Interview {
        /// Model to use (e.g., 'sonnet', 'opus', or full model name)
        #[arg(long)]
        model: Option<String>,
    },

    /// Execute the ralph loop until done or blocked
    Run {
        /// Maximum iterations before stopping
        #[arg(long, default_value = "50")]
        max_iterations: u32,

        /// Prompt for confirmation before each iteration
        #[arg(long)]
        pause: bool,

        /// Model to use (e.g., 'sonnet', 'opus', or full model name)
        #[arg(long)]
        model: Option<String>,
    },

    /// Show ralph loop progress
    Status,

    /// Remove ralph loop files
    Clean {
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init { force } => {
            init_cmd(force).await?;
        }
        Command::Interview { model } => {
            interview_cmd(model.as_deref())?;
        }
        Command::Run {
            max_iterations,
            pause,
            model,
        } => {
            run_cmd(max_iterations, pause, model.as_deref())?;
        }
        Command::Status => {
            status_cmd()?;
        }
        Command::Clean { force } => {
            clean_cmd(force)?;
        }
    }

    Ok(())
}

fn status_cmd() -> Result<()> {
    let path = Path::new(files::IMPLEMENTATION_PLAN_FILE);
    if !path.exists() {
        error::die(&format!("{} not found", files::IMPLEMENTATION_PLAN_FILE));
    }

    let content = fs::read_to_string(path)?;
    let count = parser::count_checkboxes(&content);

    println!("{}", count.render_progress_bar());

    Ok(())
}

fn clean_cmd(force: bool) -> Result<()> {
    let cwd = Path::new(".");
    let existing_files = files::find_existing_ralph_files(cwd);

    if existing_files.is_empty() {
        println!("No ralph files found.");
        return Ok(());
    }

    let file_count = existing_files.len();

    if !force {
        eprint!("Delete {} ralph files? [y/N] ", file_count);
        io::stderr().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let answer = input.trim().to_lowercase();
        if answer != "y" && answer != "yes" {
            std::process::exit(error::exit::ERROR);
        }
    }

    for path in &existing_files {
        fs::remove_file(path)?;
    }

    println!(
        "Deleted {} file{}.",
        file_count,
        if file_count == 1 { "" } else { "s" }
    );

    Ok(())
}

fn run_cmd(max_iterations: u32, pause: bool, model: Option<&str>) -> Result<()> {
    // Step 1: Validate required files exist
    run::validate_required_files()?;

    // Step 2: Read PROMPT.md
    let prompt = run::read_prompt()?;

    // Step 3: Run iteration loop
    for iteration in 1..=max_iterations {
        run::print_iteration_header(iteration);

        let result = run::spawn_claude(&prompt, model)?;

        // Log iteration output to ralph.log
        run::log_iteration(iteration, &result.stdout)?;

        if !result.success {
            error::die(&format!(
                "claude exited with code {}",
                result.exit_code.unwrap_or(-1)
            ));
        }

        // Check for completion signal in stdout
        if run::detect_done_signal(&result.stdout) == run::LoopSignal::Done {
            println!("=== Loop complete ===");
            return Ok(());
        }

        // Check for blocked signal in stdout
        if let Some(reason) = run::detect_blocked_signal(&result.stdout) {
            eprintln!("blocked: {}", reason);
            std::process::exit(error::exit::BLOCKED);
        }

        // Prompt for confirmation if --pause flag is set
        if pause && run::prompt_continue()? == run::PauseAction::Stop {
            println!("Stopped by user.");
            return Ok(());
        }
    }

    // Reached max iterations without completion
    eprintln!(
        "warning: reached max iterations ({}) without [[RALPH:DONE]]",
        max_iterations
    );
    std::process::exit(error::exit::MAX_ITERATIONS);
}

fn interview_cmd(model: Option<&str>) -> Result<()> {
    use std::process::Command;

    if !cli::claude_exists() {
        error::die("claude not found in PATH");
    }

    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| ".".to_string());

    let system_prompt = format!(
        r#"# Ralph Loop System Context

You are setting up a Ralph Loop—an autonomous development workflow where an AI agent iteratively builds software by reading local state files and executing tasks until completion.

## How the Ralph Loop Works

The `ralphctl run` command executes this loop:

1. Read PROMPT.md (orchestration instructions) and pipe it to `claude -p`
2. Claude reads SPEC.md and IMPLEMENTATION_PLAN.md to understand the project and find the next unchecked task
3. Claude implements the task, runs tests, and checks off the completed item in IMPLEMENTATION_PLAN.md
4. When done, Claude outputs `[[RALPH:DONE]]` (all tasks complete) or `[[RALPH:BLOCKED:<reason>]]` (cannot proceed)
5. If no stop signal, repeat from step 1

## Why This Architecture is Effective

**Fresh context each iteration**: Each `claude -p` invocation starts with clean context. This eliminates "context rot"—the degradation of AI performance as conversation history accumulates with stale information, abandoned approaches, and confusion.

**Local state as memory**: IMPLEMENTATION_PLAN.md checkboxes persist progress across iterations. The agent doesn't need to remember what it did—it reads the current state and determines what's next. This is more reliable than conversation-based memory.

**Atomic task execution**: Each iteration focuses on one task. Smaller, focused work produces better results than sprawling multi-task sessions.

**Stop conditions prevent waste**: `[[RALPH:DONE]]` stops the loop when all work is complete, avoiding unnecessary LLM invocations. `[[RALPH:BLOCKED:<reason>]]` stops when human intervention is needed.

## What Makes a Great SPEC.md

A spec that enables autonomous development must be:

- **Unambiguous**: No room for interpretation. "Fast" is vague; "responds within 200ms" is testable.
- **Complete**: Covers all features, edge cases, error handling, and acceptance criteria.
- **Scoped**: Clearly defines what's in and out of scope. Prevents scope creep during development.
- **Testable**: Every requirement maps to a verification method.
- **Architecturally sound**: Describes the high-level design, key components, and their interactions.

Structure:
```markdown
# Project Name

## Overview
One paragraph describing what this is and why it exists.

## Requirements
### Functional Requirements
- Specific, testable requirements

### Non-Functional Requirements
- Performance, security, reliability constraints

## Architecture
- Key components and their responsibilities
- Data flow and interactions
- Technology choices with rationale

## Out of Scope
- Explicit list of what this project does NOT do
```

## What Makes a Great IMPLEMENTATION_PLAN.md

The implementation plan is the agent's task queue. Each checkbox is one unit of work.

**Task qualities:**
- **Atomic**: Completable in one focused session (15-60 minutes of work)
- **Ordered**: Dependencies flow top-to-bottom; earlier tasks don't depend on later ones
- **Testable**: Each task has clear "done" criteria
- **Specific**: "Add user authentication" is too broad; "Implement JWT token generation in auth.rs" is specific

**Structure:**
```markdown
# Implementation Plan

## Phase 1: Foundation
- [ ] Set up project structure with Cargo.toml and module layout
- [ ] Implement core data types in src/types.rs
- [ ] Add unit tests for data types

## Phase 2: Core Features
- [ ] Implement feature X with tests
- [ ] Implement feature Y with tests

## Phase 3: Integration & Polish
- [ ] Add integration tests
- [ ] Write user documentation
```

**Phasing**: Group related tasks into phases. Complete one phase before starting the next. This provides natural checkpoints and reduces context needed per iteration.

## Interview Guidelines

Your job is to extract enough detail to write these files.

**IMPORTANT**: Always use the `AskUserQuestion` tool to ask questions. Do NOT ask questions as free-form text in your response—the user cannot reply to text responses. Every question must go through the AskUserQuestion tool so the user can provide structured answers.

Topics to cover:

1. **Core purpose**: What problem does this solve? Who is it for?
2. **Features**: What must it do? What's nice-to-have vs essential?
3. **Technical constraints**: Language, framework, dependencies, environment?
4. **Interfaces**: CLI args? API endpoints? File formats? UI?
5. **Edge cases**: What happens when things go wrong? Invalid input? Network failures?
6. **Success criteria**: How do we know it's done? What tests prove it works?
7. **Scope boundaries**: What does this explicitly NOT do?

Don't accept vague answers. "It should be fast" → "What's the latency budget? 100ms? 1s?" Push for specifics.

## After Writing the Files

When you have enough detail:

1. Write `./SPEC.md` with the complete project specification
2. Write `./IMPLEMENTATION_PLAN.md` with the phased task list
3. Summarize what you created (brief overview of the spec and number of tasks)
4. Tell the user to run `ralphctl run` to start the autonomous development loop
5. Remind them they can check progress anytime with `ralphctl status`

## Working Directory

You are working in: `{cwd}`

When writing files, use this exact path as the base. For example:
- SPEC.md → `{cwd}/SPEC.md`
- IMPLEMENTATION_PLAN.md → `{cwd}/IMPLEMENTATION_PLAN.md`

NEVER use paths from other context (like ~/.claude/CLAUDE.md). The path above is the ONLY correct location for project files."#,
        cwd = cwd
    );

    const INITIAL_PROMPT: &str = r#"You are an assistant helping me set up a Ralph Loop. Interview me to create SPEC.md and IMPLEMENTATION_PLAN.md for my project. Tell me how to get started—I might paste a detailed project idea, describe something simple, or just have a rough concept."#;

    // Launch claude in interactive mode with the interview prompt
    let mut cmd = Command::new("claude");
    cmd.arg("--allowedTools")
        .arg("AskUserQuestion,Read,Glob,Grep,Write,Edit")
        .arg("--system-prompt")
        .arg(&system_prompt);

    if let Some(m) = model {
        cmd.arg("--model").arg(m);
    }

    let status = cmd
        .arg(INITIAL_PROMPT)
        .status()
        .inspect_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                error::die("claude not found in PATH");
            }
        })?;

    if !status.success() {
        error::die(&format!(
            "claude exited with code {}",
            status.code().unwrap_or(-1)
        ));
    }

    println!();
    println!("Interview complete. Run 'ralphctl run' to start the development loop.");

    Ok(())
}

async fn init_cmd(force: bool) -> Result<()> {
    // Step 1: Verify claude CLI is in PATH
    if !cli::claude_exists() {
        error::die("claude not found in PATH");
    }

    // Step 2: Check if init files already exist
    let cwd = Path::new(".");
    let existing: Vec<_> = INIT_FILES.iter().filter(|f| cwd.join(f).exists()).collect();

    if !existing.is_empty() && !force {
        let names = existing
            .iter()
            .copied()
            .copied()
            .collect::<Vec<_>>()
            .join(", ");
        error::die(&format!(
            "files already exist: {}. Use --force to overwrite",
            names
        ));
    }

    // Step 3: Fetch templates from GitHub (with cache fallback)
    let templates = templates::get_all_templates().await?;

    // Step 4: Write files to current directory
    for (filename, content) in templates {
        fs::write(filename, content)?;
    }

    println!("Initialized ralph loop files.");
    println!();
    println!("Next steps:");
    println!("  1. Run 'ralphctl interview' to define your project interactively, or");
    println!("     manually edit SPEC.md and IMPLEMENTATION_PLAN.md");
    println!("  2. Run 'ralphctl run' to start the autonomous development loop");

    Ok(())
}
