mod cli;
mod error;
mod files;
mod parser;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

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
            println!("init (force={})", force);
        }
        Command::Run {
            max_iterations,
            pause,
        } => {
            println!("run (max_iterations={}, pause={})", max_iterations, pause);
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
