use std::time::SystemTime;
use std::io::Write;
use std::time::Duration;
use log::{debug, Record, Level};

use env_logger::{Builder, Env};
use colored::*;
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use git2::{BranchType, Repository};

mod git_utils;

#[derive(Parser)]
#[command(name = "purgit")]
#[command(about = "A git helper CLI", long_about = None)]
struct Cli {
    #[arg(short, long, global = true)]
    quiet: bool,

    #[arg(short, long, global = true)]
    verbose: bool,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Clean {
        #[arg(short, long)]
        yes: bool,

        #[arg(long, default_value_t = 30)]
        stale: u64,
    },
}

#[derive(Debug)]
struct BranchDetails {
    name: String,
    kind: String,
    age: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.verbose {
        // Set up logging
        Builder::from_env(Env::default().default_filter_or("debug"))
            .format(|buf, record: &Record| {
                let level = match record.level() {
                    Level::Error => "ERROR".red(),
                    Level::Warn  => "WARN".yellow(),
                    Level::Info  => "INFO".green(),
                    Level::Debug => "DEBUG".blue(),
                    Level::Trace => "TRACE".magenta(),
                };

                writeln!(buf, "[{}] {}", level, record.args())
            })
            .format_timestamp(None)
            .format_target(false)
            .init();
    }

    match &cli.command {
        Commands::Clean {stale, yes} => {
            // Initialize progress bar if not quiet or verbose
            let progress = if !(cli.quiet || cli.verbose) {
                Some(ProgressBar::new_spinner())
            } else { 
                None 
            };

            // Set up progress bar
            if let Some(ref progress) = progress {
                progress.set_message("Fetching...");
                progress.enable_steady_tick(Duration::from_millis(100));
                progress.set_style(ProgressStyle::default_spinner()
                    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                    .template("{spinner} {msg}")
                    .expect("Invalid template"));
            }
            
            let repo = Repository::open(".").expect("No Git repository found in current directory.");
            git_utils::fetch_remote(&repo, "origin")?;

            if let Some(ref progress) = progress {
                progress.set_message("Scanning branches...");
            }

            let mut branches = Vec::new();
            for branch_result in repo.branches(None)? {
                let (branch, branch_type) = branch_result?;

                let name = branch.name()?.unwrap_or("<invalid UTF-8>");
                let kind = match branch_type {
                    BranchType::Local => "local",
                    BranchType::Remote => "remote",
                };

                let commit = branch.get().target().and_then(|oid| repo.find_commit(oid).ok());

                if let Some(commit) = commit {
                    let commit_time = commit.time().seconds() as u64;
                    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as u64;
                    let age = Duration::from_secs(now - commit_time).as_secs() / 86400;
                    if age > *stale {
                        branches.push(BranchDetails { name: name.to_string(), kind: kind.to_string(), age: age });
                    }
                }

                debug!("Found {}:{} branch.", kind, name);
            }
            
            if let Some(ref progress) = progress {
                progress.finish_and_clear();
            }

            branches.sort_by(|a, b| b.age.cmp(&a.age));

            let max_name_len = branches
                .iter()
                .map(|b| b.name.len())
                .max()
                .unwrap_or(10);

            if !cli.quiet {
                println!("Found {} stale branches.", branches.len());
                for branch in &branches {
                    let branch_str = format!("{:<width$}", branch.name, width = max_name_len).green();
                    let age_str = format!("{}d", branch.age).blue();
                    println!(
                        "* {}    {}",
                        branch_str,
                        age_str,
                    );
                }
            }
        }
    }

    Ok(())
}