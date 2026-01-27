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

    /// Execute the ralph loop until done or blocked
    Run {
        /// Maximum iterations before stopping
        #[arg(long, default_value = "50")]
        max_iterations: u32,

        /// Prompt for confirmation before each iteration
        #[arg(long)]
        pause: bool,
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
        Command::Run {
            max_iterations,
            pause,
        } => {
            run_cmd(max_iterations, pause)?;
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

fn run_cmd(max_iterations: u32, _pause: bool) -> Result<()> {
    // Step 1: Validate required files exist
    run::validate_required_files()?;

    // Step 2: Read PROMPT.md
    let prompt = run::read_prompt()?;

    // Step 3: Run iteration loop
    // TODO: Remove allow when magic string detection is implemented
    #[allow(clippy::never_loop)]
    for iteration in 1..=max_iterations {
        // Print iteration header
        run::print_iteration_header(iteration);

        // Spawn claude subprocess
        let result = run::spawn_claude(&prompt)?;

        // TODO: Check for magic strings (RALPH:DONE, RALPH:BLOCKED)
        // TODO: Log iteration to ralph.log
        // TODO: Handle --pause flag

        // For now, exit after first iteration if claude exited with error
        if !result.success {
            error::die(&format!(
                "claude exited with code {}",
                result.exit_code.unwrap_or(-1)
            ));
        }

        // TODO: Remove this break when magic string detection is implemented
        break;
    }

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

    Ok(())
}
