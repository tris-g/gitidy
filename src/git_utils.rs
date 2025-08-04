use std::path::PathBuf;
use log::debug;
use git2::{Repository, RemoteCallbacks, FetchOptions, Cred, AutotagOption};

/// Resolve the Git repository name using its file path.
pub fn resolve_name(repo: &Repository) -> Result<String, Box<dyn std::error::Error>> {
    let repo_path: PathBuf = repo.path().canonicalize()?;

    let repo_root = repo_path
        .parent()
        .and_then(|p| p.file_name())
        .ok_or("Could not determine repository name")?;
    
    debug!("Resolved repository name.");
    Ok(repo_root.to_string_lossy().into_owned())
}

/// Fetches all updates from the specified remote of the given Git repository.
///
/// This function performs a full `git fetch` operation:
/// - Downloads all branches and tags
/// - Updates local remote-tracking branches
/// - Prunes any deleted remote branches
/// - Updates `.git/FETCH_HEAD`
///
/// Authentication is handled using Git's configured credential helpers.
pub fn fetch_remote(repo: &Repository, remote_name: &str) -> Result<(), git2::Error> {
    debug!("Setting up remote fetch options...");
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
    debug!("Fetched from remote.");
    Ok(())
}