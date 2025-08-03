use std::{path::PathBuf, time::SystemTime};
use std::io::Write;
use std::time::Duration;
use log::{debug, Record, Level};

use env_logger::{Builder, Env};
use colored::*;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use git2::{BranchType, Repository, FetchOptions, RemoteCallbacks, Cred, AutotagOption};

fn get_repo_name(repo: &Repository) -> String {
    let result = || -> Result<String, Box<dyn std::error::Error>> {
        let repo_path: PathBuf = repo.path().canonicalize()?;

        let repo_root = repo_path
            .parent()
            .and_then(|p| p.file_name())
            .ok_or_else(|| Box::<dyn std::error::Error>::from("Could not determine repository name"))?;

        Ok(repo_root.to_string_lossy().into_owned())
    };

    result().unwrap_or_else(|_| "Unknown".to_string())
}

/// Completely updates the given repository from the origin.
pub fn fetch_all(repo: &Repository, remote_name: &str) -> Result<(), git2::Error> {
    let mut remote = repo.find_remote(remote_name)?;
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(move |url, username_from_url, _| {
        Cred::credential_helper(&repo.config()?, url, username_from_url)
    });
    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);
    fetch_options.download_tags(AutotagOption::All);
    fetch_options.update_fetchhead(true);
    fetch_options.prune(git2::FetchPrune::On);
    remote.fetch(&[] as &[&str], Some(&mut fetch_options), None)?;

    Ok(())
}

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

        debug!("Verbose mode enabled");
    }

    // TODO: Disable spinner if verbose enabled

    let spinner = ProgressBar::new_spinner();
    spinner.set_message("Fetching...");
    spinner.enable_steady_tick(Duration::from_millis(100));

    spinner.set_style(ProgressStyle::default_spinner()
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
        .template("{spinner} {msg}")
        .expect("Invalid template"));

    let repo = Repository::open(".").expect("Not a Git repository");
    fetch_all(&repo, "origin")?;
    spinner.set_message("Scanning branches...");

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
            
           debug!("Found {} branch named {} that is {} days old.", kind, name, age);
        }
    }
    
    spinner.finish_and_clear();
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