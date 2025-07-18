use crate::error::{GhExportError, Result};
use crate::github::Repository;
use crate::progress::ProgressTracker;
use futures::StreamExt;
use git2::{Cred, FetchOptions, RemoteCallbacks};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, error, info};

pub struct Downloader {
    output_dir: PathBuf,
    token: String,
    shallow: bool,
    progress: Arc<ProgressTracker>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum DownloadResult {
    Success,
    Skipped(String),
    Failed(String),
}

impl Downloader {
    pub fn new(
        output_dir: PathBuf,
        token: String,
        shallow: bool,
        progress: Arc<ProgressTracker>,
    ) -> Self {
        Self {
            output_dir,
            token,
            shallow,
            progress,
        }
    }

    pub async fn download_repositories(
        &self,
        repositories: Vec<Repository>,
        max_concurrent: usize,
    ) -> Result<Vec<(String, DownloadResult)>> {
        let semaphore = Arc::new(Semaphore::new(max_concurrent));
        let mut tasks = Vec::new();

        for repo in repositories {
            let semaphore = semaphore.clone();
            let downloader = self.clone_for_task();

            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                let result = downloader.download_repository(&repo).await;
                (repo.full_name.clone(), result)
            });

            tasks.push(task);
        }

        let mut results = Vec::new();
        let mut task_stream = futures::stream::iter(tasks);

        while let Some(task) = task_stream.next().await {
            match task.await {
                Ok((name, result)) => results.push((name, result)),
                Err(e) => {
                    error!("Task join error: {}", e);
                }
            }
        }

        Ok(results)
    }

    async fn download_repository(&self, repo: &Repository) -> DownloadResult {
        let repo_path = self.output_dir.join(&repo.owner.login).join(&repo.name);

        if repo_path.exists() {
            debug!("Repository {} already exists, updating...", repo.full_name);
            match self.update_repository(&repo_path).await {
                Ok(_) => {
                    self.progress.increment_completed();
                    DownloadResult::Success
                }
                Err(e) => {
                    self.progress.increment_failed();
                    DownloadResult::Failed(format!("Update failed: {e}"))
                }
            }
        } else {
            info!("Cloning repository {}", repo.full_name);
            match self.clone_repository(repo, &repo_path).await {
                Ok(_) => {
                    self.progress.increment_completed();
                    DownloadResult::Success
                }
                Err(e) => {
                    self.progress.increment_failed();
                    DownloadResult::Failed(format!("Clone failed: {e}"))
                }
            }
        }
    }

    async fn clone_repository(&self, repo: &Repository, target_path: &Path) -> Result<()> {
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let token = self.token.clone();
        let clone_url = repo.clone_url.clone();
        let target_path = target_path.to_path_buf();
        let shallow = self.shallow;
        let progress = self.progress.clone();
        let repo_name = repo.name.clone();

        tokio::task::spawn_blocking(move || {
            let mut callbacks = RemoteCallbacks::new();
            callbacks.credentials(|_url, username_from_url, _allowed_types| {
                Cred::userpass_plaintext(username_from_url.unwrap_or("git"), &token)
            });

            callbacks.transfer_progress(|stats| {
                let received = stats.received_objects();
                let total = stats.total_objects();
                if total > 0 {
                    progress.update_repo_progress(&repo_name, received as u32, total as u32);
                }
                true
            });

            let mut fetch_options = FetchOptions::new();
            fetch_options.remote_callbacks(callbacks);

            if shallow {
                fetch_options.depth(1);
            }

            let mut builder = git2::build::RepoBuilder::new();
            builder.fetch_options(fetch_options);

            builder
                .clone(&clone_url, &target_path)
                .map_err(GhExportError::Git)?;

            Ok(())
        })
        .await
        .map_err(|e| GhExportError::Download(format!("Clone task failed: {e}")))?
    }

    async fn update_repository(&self, repo_path: &Path) -> Result<()> {
        let token = self.token.clone();
        let repo_path = repo_path.to_path_buf();

        tokio::task::spawn_blocking(move || {
            let repo = git2::Repository::open(&repo_path)?;
            let mut remote = repo.find_remote("origin")?;

            let mut callbacks = RemoteCallbacks::new();
            callbacks.credentials(|_url, username_from_url, _allowed_types| {
                Cred::userpass_plaintext(username_from_url.unwrap_or("git"), &token)
            });

            let mut fetch_options = FetchOptions::new();
            fetch_options.remote_callbacks(callbacks);

            remote.fetch(
                &["refs/heads/*:refs/remotes/origin/*"],
                Some(&mut fetch_options),
                None,
            )?;

            let fetch_head = repo.find_reference("FETCH_HEAD")?;
            let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;

            let analysis = repo.merge_analysis(&[&fetch_commit])?;

            if analysis.0.is_fast_forward() {
                let refname = "refs/heads/master";
                match repo.find_reference(refname) {
                    Ok(mut reference) => {
                        reference.set_target(fetch_commit.id(), "Fast-forward")?;
                        repo.set_head(refname)?;
                        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
                    }
                    Err(_) => {
                        let refname = "refs/heads/main";
                        if let Ok(mut reference) = repo.find_reference(refname) {
                            reference.set_target(fetch_commit.id(), "Fast-forward")?;
                            repo.set_head(refname)?;
                            repo.checkout_head(Some(
                                git2::build::CheckoutBuilder::default().force(),
                            ))?;
                        }
                    }
                }
            }

            Ok(())
        })
        .await
        .map_err(|e| GhExportError::Download(format!("Update task failed: {e}")))?
    }

    fn clone_for_task(&self) -> Self {
        Self {
            output_dir: self.output_dir.clone(),
            token: self.token.clone(),
            shallow: self.shallow,
            progress: self.progress.clone(),
        }
    }
}

pub async fn check_disk_space(path: &Path, required_bytes: u64) -> Result<()> {
    #[cfg(unix)]
    {
        let stat = nix::sys::statvfs::statvfs(path)
            .map_err(|e| GhExportError::Io(std::io::Error::other(e)))?;

        let available = stat.blocks_available() as u64 * stat.block_size();

        if available < required_bytes {
            return Err(GhExportError::InsufficientSpace {
                needed: required_bytes,
                available,
            });
        }
    }

    Ok(())
}
