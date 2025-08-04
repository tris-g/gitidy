use std::time::SystemTime;
use std::io::Write;
use std::time::Duration;
use log::{debug, Record, Level};

use env_logger::{Builder, Env};
use colored::*;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use git2::{BranchType, Repository};

mod git_utils;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    verbose: bool,

    #[arg(short, long)]
    yes: bool,
}

#[derive(Debug)]
struct BranchAge {
    name: String,
    age: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.verbose {
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

    // Initialize progress bar if not verbose
    let progress = if !args.verbose {
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

    let mut branch_ages = Vec::new();
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
            branch_ages.push(BranchAge { name: name.to_string(), age: age });
        }

        debug!("Found {}:{} branch.", kind, name);
    }
    
    if let Some(ref progress) = progress {
        progress.finish_and_clear();
    }

    branch_ages.sort_by(|a, b| b.age.cmp(&a.age));
    let max_name_len = branch_ages.iter().map(|b| b.name.len()).max().unwrap_or(10);

    println!("{:<width$}  {}", "Branch", "Age (days)", width = max_name_len);
    println!("{:-<width$}  {:-<10}", "", "", width = max_name_len);

    for branch in branch_ages {
        println!(
            "{:<width$}  {:>10}",
            branch.name,
            branch.age,
            width = max_name_len
        );
    }

    Ok(())
}