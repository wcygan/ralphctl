mod cli;
mod error;
mod files;
mod parser;
mod reverse;
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
#[command(version)]
#[command(about = "Manage Ralph Loop workflows—autonomous development sessions driven by Claude")]
#[command(after_help = "\
WORKFLOW:
  init      → Scaffold template files (SPEC.md, IMPLEMENTATION_PLAN.md, PROMPT.md)
  interview → AI-guided session to fill out SPEC.md and IMPLEMENTATION_PLAN.md
  run       → Execute the autonomous development loop
  status    → Check progress at any time
  archive   → Save completed work and reset for next project
  clean     → Remove all ralph files when done

EXAMPLES:
  ralphctl init                  # Start a new ralph loop
  ralphctl interview             # Define your project interactively
  ralphctl run                   # Execute until done or blocked
  ralphctl run --pause           # Step through iterations manually
  ralphctl status                # Check task completion progress
  ralphctl archive               # Save spec/plan and reset to blank
  ralphctl fetch-latest-prompt   # Update PROMPT.md to latest version
")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scaffold ralph loop files from GitHub templates
    #[command(
        long_about = "Fetch template files from GitHub and create them in the current directory.\n\n\
                      Creates: SPEC.md, IMPLEMENTATION_PLAN.md, PROMPT.md\n\n\
                      Templates are cached locally for offline use. Requires the claude CLI to be installed.",
        after_help = "EXAMPLES:\n  ralphctl init           # Create files (fails if they exist)\n  ralphctl init --force   # Overwrite existing files"
    )]
    Init {
        /// Overwrite existing files without prompting
        #[arg(long)]
        force: bool,
    },

    /// AI-guided interview to create SPEC.md and IMPLEMENTATION_PLAN.md
    #[command(
        long_about = "Launch an interactive Claude session to define your project.\n\n\
                      Claude will ask questions about your project's purpose, requirements,\n\
                      architecture, and scope, then generate SPEC.md and IMPLEMENTATION_PLAN.md.",
        after_help = "EXAMPLES:\n  ralphctl interview              # Use default model\n  ralphctl interview --model opus # Use a specific model"
    )]
    Interview {
        /// Claude model to use (e.g., 'sonnet', 'opus', or full model name)
        #[arg(long, value_name = "MODEL")]
        model: Option<String>,
    },

    /// Execute the ralph loop until done or blocked
    #[command(
        long_about = "Run the autonomous development loop by piping PROMPT.md to claude.\n\n\
                      Each iteration: Claude reads state files, implements one task, marks it complete.\n\
                      Loop ends when [[RALPH:DONE]] or [[RALPH:BLOCKED:<reason>]] is detected.",
        after_help = "EXIT CODES:\n  \
                      0   Success (RALPH:DONE detected)\n  \
                      1   Error or RALPH:BLOCKED detected\n  \
                      2   Max iterations reached\n  \
                      130 Interrupted (Ctrl+C)\n\n\
                      EXAMPLES:\n  \
                      ralphctl run                      # Run up to 50 iterations\n  \
                      ralphctl run --max-iterations 10  # Limit to 10 iterations\n  \
                      ralphctl run --pause              # Confirm before each iteration\n  \
                      ralphctl run --model opus         # Use a specific model"
    )]
    Run {
        /// Maximum iterations before stopping
        #[arg(long, default_value = "50", value_name = "N")]
        max_iterations: u32,

        /// Prompt for confirmation before each iteration
        #[arg(long)]
        pause: bool,

        /// Claude model to use (e.g., 'sonnet', 'opus', or full model name)
        #[arg(long, value_name = "MODEL")]
        model: Option<String>,
    },

    /// Show ralph loop progress from IMPLEMENTATION_PLAN.md
    #[command(
        long_about = "Parse IMPLEMENTATION_PLAN.md and display a progress bar showing task completion.\n\n\
                      Counts all checkboxes (- [ ] and - [x]) to calculate percentage complete.",
        after_help = "OUTPUT FORMAT:\n  [████████░░░░] 60% (12/20 tasks)"
    )]
    Status,

    /// Remove ralph loop files
    #[command(
        long_about = "Delete all ralph-related files from the current directory.\n\n\
                      Files removed: SPEC.md, IMPLEMENTATION_PLAN.md, PROMPT.md, ralph.log",
        after_help = "EXAMPLES:\n  ralphctl clean          # Prompt for confirmation\n  ralphctl clean --force  # Delete without prompting"
    )]
    Clean {
        /// Delete files without confirmation prompt
        #[arg(long)]
        force: bool,
    },

    /// Archive SPEC.md and IMPLEMENTATION_PLAN.md, then reset to blank
    #[command(
        long_about = "Save the current SPEC.md and IMPLEMENTATION_PLAN.md to a timestamped archive\n\
                      directory (.ralphctl/archive/<timestamp>/), then reset them to blank templates.\n\n\
                      Useful for starting a new project while preserving completed work.",
        after_help = "EXAMPLES:\n  ralphctl archive          # Prompt for confirmation\n  ralphctl archive --force  # Archive without prompting"
    )]
    Archive {
        /// Archive files without confirmation prompt
        #[arg(long)]
        force: bool,
    },

    /// Update ralphctl to the latest version from GitHub
    #[command(
        long_about = "Install the latest version of ralphctl from GitHub using cargo.\n\n\
                      Runs: cargo install --git https://github.com/wcygan/ralphctl"
    )]
    Update,

    /// Fetch the latest PROMPT.md from GitHub
    #[command(
        long_about = "Fetch the latest PROMPT.md from GitHub without affecting other files.\n\n\
                      Use this when the Ralph Loop protocol has been updated with new control signals\n\
                      or improved orchestration logic. Your SPEC.md and IMPLEMENTATION_PLAN.md remain untouched.",
        after_help = "WHY USE THIS:\n\
                      The PROMPT.md file contains the orchestration instructions for Claude, including\n\
                      magic control signals like [[RALPH:DONE]] and [[RALPH:BLOCKED:<reason>]]. When\n\
                      ralphctl is updated with new signals or improved prompting, running this command\n\
                      ensures your local prompt stays current.\n\n\
                      EXAMPLES:\n  ralphctl fetch-latest-prompt    # Download latest PROMPT.md"
    )]
    FetchLatestPrompt,

    /// Investigate a codebase to answer a question
    #[command(
        long_about = "Run an autonomous investigation loop to answer a question about the codebase.\n\n\
                      Unlike 'run' which builds software, 'reverse' analyzes code to answer questions—\n\
                      diagnosing bugs, understanding systems, or mapping dependencies before changes.\n\n\
                      Creates: QUESTION.md (from argument or template), INVESTIGATION.md, FINDINGS.md",
        after_help = "EXAMPLES:\n  \
                      ralphctl reverse \"Why does auth fail?\"      # Provide question directly\n  \
                      ralphctl reverse                             # Use existing QUESTION.md\n  \
                      ralphctl reverse --model opus \"How?\"        # Use specific model\n  \
                      ralphctl reverse --pause                     # Confirm each iteration\n\n\
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

        /// Claude model to use (e.g., 'sonnet', 'opus', or full model name)
        #[arg(long, value_name = "MODEL")]
        model: Option<String>,
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
        Command::Archive { force } => {
            archive_cmd(force)?;
        }
        Command::Update => {
            update_cmd()?;
        }
        Command::FetchLatestPrompt => {
            fetch_latest_prompt_cmd().await?;
        }
        Command::Reverse {
            question,
            max_iterations,
            pause,
            model,
        } => {
            reverse_cmd(question, max_iterations, pause, model.as_deref()).await?;
        }
    }

    Ok(())
}

fn update_cmd() -> Result<()> {
    use std::process::Command;

    println!("Updating ralphctl...");

    let status = Command::new("cargo")
        .args(["install", "--git", "https://github.com/wcygan/ralphctl"])
        .status()?;

    if !status.success() {
        error::die(&format!(
            "cargo install failed with code {}",
            status.code().unwrap_or(-1)
        ));
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

fn archive_cmd(force: bool) -> Result<()> {
    let cwd = Path::new(".");
    let archivable_files = files::find_archivable_files(cwd);

    if archivable_files.is_empty() {
        println!("No archivable files found.");
        return Ok(());
    }

    let file_count = archivable_files.len();

    if !force {
        eprint!(
            "Archive {} file{}? [y/N] ",
            file_count,
            if file_count == 1 { "" } else { "s" }
        );
        io::stderr().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let answer = input.trim().to_lowercase();
        if answer != "y" && answer != "yes" {
            std::process::exit(error::exit::ERROR);
        }
    }

    // Ensure .ralphctl is in .gitignore
    update_gitignore(cwd)?;

    // Create timestamped archive directory
    let timestamp = generate_timestamp();
    let archive_dir = files::archive_base_dir(cwd).join(&timestamp);
    fs::create_dir_all(&archive_dir)?;

    // Copy files to archive
    for path in &archivable_files {
        let filename = path.file_name().unwrap();
        let dest = archive_dir.join(filename);
        fs::copy(path, dest)?;
    }

    // Reset original files to blank templates
    for path in &archivable_files {
        let blank = generate_blank_content(path);
        fs::write(path, blank)?;
    }

    println!(
        "Archived {} file{} to {}",
        file_count,
        if file_count == 1 { "" } else { "s" },
        archive_dir.display()
    );

    Ok(())
}

/// Generate a filesystem-safe timestamp for archive directories.
fn generate_timestamp() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H-%M-%S").to_string()
}

/// Generate blank content for a given file.
fn generate_blank_content(path: &Path) -> &'static str {
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    match filename {
        files::SPEC_FILE => "# Specification\n\n",
        files::IMPLEMENTATION_PLAN_FILE => "# Implementation Plan\n\n",
        _ => "",
    }
}

/// Update .gitignore to include .ralphctl if not already present.
fn update_gitignore(dir: &Path) -> Result<()> {
    let gitignore_path = dir.join(".gitignore");
    let entry = files::RALPHCTL_DIR;

    if gitignore_path.exists() {
        let content = fs::read_to_string(&gitignore_path)?;
        // Check if entry already exists (as a complete line)
        if content.lines().any(|line| line.trim() == entry) {
            return Ok(());
        }
        // Append entry with newline handling
        let suffix = if content.ends_with('\n') || content.is_empty() {
            format!("{}\n", entry)
        } else {
            format!("\n{}\n", entry)
        };
        fs::write(&gitignore_path, content + &suffix)?;
    } else {
        fs::write(&gitignore_path, format!("{}\n", entry))?;
    }

    Ok(())
}

fn run_cmd(max_iterations: u32, pause: bool, model: Option<&str>) -> Result<()> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    // Step 1: Validate required files exist
    run::validate_required_files()?;

    // Step 2: Read PROMPT.md
    let prompt = run::read_prompt()?;

    // Step 3: Set up Ctrl+C handler
    let interrupt_flag = Arc::new(AtomicBool::new(false));
    let interrupt_flag_clone = interrupt_flag.clone();

    ctrlc::set_handler(move || {
        interrupt_flag_clone.store(true, Ordering::SeqCst);
    })
    .expect("error setting Ctrl+C handler");

    // Step 4: Run iteration loop
    let mut iterations_completed = 0u32;

    for iteration in 1..=max_iterations {
        run::print_iteration_header(iteration);

        let result = run::spawn_claude(&prompt, model, Some(interrupt_flag.clone()))?;

        // Log iteration output to ralph.log
        run::log_iteration(iteration, &result.stdout)?;

        // Check if we were interrupted
        if result.was_interrupted {
            run::print_interrupt_summary(iterations_completed);
            std::process::exit(error::exit::INTERRUPTED);
        }

        iterations_completed = iteration;

        if !result.success {
            error::die(&format!(
                "claude exited with code {}",
                result.exit_code.unwrap_or(-1)
            ));
        }

        // Check for blocked signal first (takes priority)
        if let Some(reason) = run::detect_blocked_signal(&result.stdout) {
            eprintln!("blocked: {}", reason);
            std::process::exit(error::exit::BLOCKED);
        }

        // Check for completion/continue signals in stdout
        match run::detect_signal(&result.stdout) {
            run::LoopSignal::Done => {
                println!("=== Loop complete ===");
                return Ok(());
            }
            run::LoopSignal::Continue => {
                // Task completed, continue to next iteration
                // If --pause is set, prompt user before continuing
                if pause && run::prompt_continue()? == run::PauseAction::Stop {
                    println!("Stopped by user.");
                    return Ok(());
                }
            }
            run::LoopSignal::NoSignal => {
                // No signal detected, prompt user for action
                if !pause && run::prompt_no_signal()? == run::NoSignalAction::Stop {
                    println!("Stopped by user.");
                    return Ok(());
                }
                // If --pause is set, that prompt handles continuation
                if pause && run::prompt_continue()? == run::PauseAction::Stop {
                    println!("Stopped by user.");
                    return Ok(());
                }
            }
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

    let status = cmd.arg(INITIAL_PROMPT).status().inspect_err(|e| {
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

async fn fetch_latest_prompt_cmd() -> Result<()> {
    let content = templates::get_template("PROMPT.md").await?;
    fs::write("PROMPT.md", content)?;
    println!("Updated PROMPT.md to latest version.");
    Ok(())
}

async fn reverse_cmd(
    question: Option<String>,
    max_iterations: u32,
    pause: bool,
    model: Option<&str>,
) -> Result<()> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let cwd = Path::new(".");

    // Step 1: Handle question setup
    // - If argument provided: write to QUESTION.md
    // - If no argument and QUESTION.md exists: use existing file
    // - If no argument and no QUESTION.md: create template, print instructions, exit
    if let Some(q) = question {
        reverse::write_question(cwd, &q)?;
    } else if !cwd.join(files::QUESTION_FILE).exists() {
        reverse::create_question_template(cwd)?;
        eprintln!(
            "Created {}. Edit it with your investigation question, then run 'ralphctl reverse' again.",
            files::QUESTION_FILE
        );
        std::process::exit(error::exit::ERROR);
    }

    // Step 2: Verify claude CLI exists
    if !cli::claude_exists() {
        error::die("claude not found in PATH");
    }

    // Step 3: Fetch REVERSE_PROMPT.md template
    let prompt = templates::get_reverse_template().await?;

    // Write REVERSE_PROMPT.md to current directory for reference
    fs::write(files::REVERSE_PROMPT_FILE, &prompt)?;

    // Step 4: Set up Ctrl+C handler
    let interrupt_flag = Arc::new(AtomicBool::new(false));
    let interrupt_flag_clone = interrupt_flag.clone();

    ctrlc::set_handler(move || {
        interrupt_flag_clone.store(true, Ordering::SeqCst);
    })
    .expect("error setting Ctrl+C handler");

    // Step 5: Run investigation loop
    let mut iterations_completed = 0u32;

    for iteration in 1..=max_iterations {
        run::print_iteration_header(iteration);

        // Handle pause mode
        if pause && run::prompt_continue()? == run::PauseAction::Stop {
            println!("Stopped by user.");
            return Ok(());
        }

        let result = run::spawn_claude(&prompt, model, Some(interrupt_flag.clone()))?;

        // Log iteration output to ralph.log
        run::log_iteration(iteration, &result.stdout)?;

        // Check if we were interrupted
        if result.was_interrupted {
            print_reverse_interrupt_summary(iterations_completed);
            std::process::exit(error::exit::INTERRUPTED);
        }

        iterations_completed = iteration;

        if !result.success {
            error::die(&format!(
                "claude exited with code {}",
                result.exit_code.unwrap_or(-1)
            ));
        }

        // Detect reverse mode signals (priority: BLOCKED → FOUND → INCONCLUSIVE → CONTINUE)
        match reverse::detect_reverse_signal(&result.stdout) {
            reverse::ReverseSignal::Blocked(reason) => {
                eprintln!("blocked: {}", reason);
                std::process::exit(error::exit::BLOCKED);
            }
            reverse::ReverseSignal::Found(summary) => {
                println!("=== Investigation complete ===");
                println!("Found: {}", summary);
                return Ok(());
            }
            reverse::ReverseSignal::Inconclusive(reason) => {
                eprintln!("=== Investigation inconclusive ===");
                eprintln!("{}", reason);
                std::process::exit(error::exit::INCONCLUSIVE);
            }
            reverse::ReverseSignal::Continue => {
                // Still investigating, continue to next iteration
            }
            reverse::ReverseSignal::NoSignal => {
                // No signal detected, prompt user for action
                if run::prompt_no_signal()? == run::NoSignalAction::Stop {
                    println!("Stopped by user.");
                    return Ok(());
                }
            }
        }
    }

    // Reached max iterations without completion
    eprintln!(
        "warning: reached max iterations ({}) without finding an answer",
        max_iterations
    );
    std::process::exit(error::exit::MAX_ITERATIONS);
}

/// Print interrupt summary for reverse mode.
fn print_reverse_interrupt_summary(iterations_completed: u32) {
    eprintln!(
        "Interrupted after {} iteration{}.",
        iterations_completed,
        if iterations_completed == 1 { "" } else { "s" }
    );
}
