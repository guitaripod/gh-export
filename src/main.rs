mod cli;
mod config;
mod download;
mod error;
mod github;
mod progress;

use clap::Parser;
use cli::{Cli, Commands, ConfigAction};
use config::{Config, ExportMetadata};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Password};
use download::{check_disk_space, DownloadResult, Downloader};
use error::Result;
use github::GitHubClient;
use progress::{create_spinner, ProgressTracker};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    let mut cli = Cli::parse();

    let log_level = if cli.verbose {
        "debug"
    } else if cli.quiet {
        "error"
    } else {
        "info"
    };

    tracing_subscriber::fmt().with_env_filter(log_level).init();

    match cli.command.take() {
        Some(Commands::Config { action }) => handle_config_command(action).await,
        Some(Commands::Status) => handle_status_command().await,
        Some(Commands::Sync { since }) => handle_sync_command(cli, since).await,
        None => handle_export_command(cli).await,
    }
}

async fn handle_config_command(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show => {
            let config = Config::load()?;
            println!("{}", style("Current configuration:").bold());
            println!(
                "  Token: {}",
                if config.github_token.is_some() {
                    "***"
                } else {
                    "Not set"
                }
            );
            println!("  Output directory: {}", config.output_directory.display());
            println!("  Parallel downloads: {}", config.parallel_downloads);
            println!("  Include archived: {}", config.include_archived);
            println!("  Exclude forks: {}", config.exclude_forks);
            println!("  Shallow clone: {}", config.shallow_clone);
            Ok(())
        }
        ConfigAction::Set { key, value } => {
            let mut config = Config::load()?;

            match key.as_str() {
                "token" => config.github_token = Some(value),
                "output" => config.output_directory = value.into(),
                "parallel" => {
                    config.parallel_downloads = value.parse().map_err(|_| {
                        error::GhExportError::Config("Invalid parallel value".to_string())
                    })?;
                }
                _ => {
                    return Err(error::GhExportError::Config(format!(
                        "Unknown configuration key: {key}"
                    )))
                }
            }

            config.save()?;
            println!("{}", style("Configuration updated successfully").green());
            Ok(())
        }
        ConfigAction::Clear => {
            if Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Are you sure you want to clear all configuration?")
                .default(false)
                .interact()?
            {
                std::fs::remove_file(Config::config_path()?)?;
                println!("{}", style("Configuration cleared").green());
            }
            Ok(())
        }
    }
}

async fn handle_status_command() -> Result<()> {
    let config = Config::load()?;

    if let Some(metadata) = ExportMetadata::load(&config.output_directory)? {
        println!("{}", style("Last export information:").bold());
        println!(
            "  Date: {}",
            metadata.last_export.format("%Y-%m-%d %H:%M:%S UTC")
        );
        println!("  Total repositories: {}", metadata.total_repos);
        println!("  Successful: {}", metadata.successful_exports);
        println!("  Failed: {}", metadata.failed_exports.len());

        if !metadata.failed_exports.is_empty() {
            println!("\n{}", style("Failed repositories:").red());
            for repo in &metadata.failed_exports {
                println!("  - {repo}");
            }
        }

        let duration = chrono::Duration::seconds(metadata.export_duration_seconds as i64);
        println!("\n  Duration: {}", format_duration(duration));
    } else {
        println!("{}", style("No export information found").yellow());
    }

    Ok(())
}

async fn handle_sync_command(cli: Cli, since: Option<String>) -> Result<()> {
    let mut config = Config::load()?;
    merge_cli_config(&mut config, &cli);

    if config.github_token.is_none() {
        return Err(error::GhExportError::Config(
            "No GitHub token configured. Run without subcommand to set up.".to_string(),
        ));
    }

    println!("{}", style("Syncing repositories...").bold());
    if let Some(since) = since {
        println!("Only updating repositories modified after: {since}");
    }

    run_export(config, true).await
}

async fn handle_export_command(cli: Cli) -> Result<()> {
    let mut config = Config::load()?;
    merge_cli_config(&mut config, &cli);

    if config.github_token.is_none() && !cli.quiet {
        println!("{}", style("Welcome to GitHub Export!").bold().green());
        println!("\nThis tool will help you export all repositories from your GitHub account.\n");

        if !Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Do you have a GitHub personal access token?")
            .default(true)
            .interact()?
        {
            println!("\nTo create a token:");
            println!("1. Go to https://github.com/settings/tokens");
            println!("2. Click 'Generate new token' → 'Generate new token (classic)'");
            println!("3. Give it a name (e.g., 'gh-export')");
            println!(
                "4. Select scopes: 'repo' (for private repos) or 'public_repo' (for public only)"
            );
            println!("5. Click 'Generate token' and copy it\n");
        }

        let token = Password::with_theme(&ColorfulTheme::default())
            .with_prompt("GitHub token (will be stored securely)")
            .interact()?;

        config.github_token = Some(token);

        let default_output = config.output_directory.display().to_string();
        let output = Input::<String>::with_theme(&ColorfulTheme::default())
            .with_prompt("Where to save repositories?")
            .default(default_output)
            .interact()?;

        config.output_directory = output.into();
        config.save()?;

        println!("\n{}", style("Configuration saved!").green());
    }

    if config.github_token.is_none() {
        return Err(error::GhExportError::Config(
            "No GitHub token provided".to_string(),
        ));
    }

    run_export(config, false).await
}

async fn run_export(config: Config, is_sync: bool) -> Result<()> {
    config.validate()?;
    config.ensure_output_directory()?;

    let start_time = Instant::now();
    let client = GitHubClient::new(config.github_token.clone().unwrap())?;

    let spinner = create_spinner("Checking authentication...");
    let user = client.get_authenticated_user().await?;
    spinner.finish_and_clear();

    println!(
        "{} {}",
        style("Authenticated as:").bold(),
        style(&user.login).cyan()
    );

    let spinner = create_spinner("Fetching repository list...");
    let mut repositories = client.list_user_repositories(&user.login).await?;
    spinner.finish_and_clear();

    if !config.include_archived {
        repositories.retain(|repo| !repo.archived);
    }

    if config.exclude_forks {
        repositories.retain(|repo| !repo.fork);
    }

    let total_size: u64 = repositories.iter().map(|r| r.size * 1024).sum();

    println!(
        "{} {} repositories (estimated size: {})",
        style("Found").bold(),
        style(repositories.len()).cyan(),
        style(format_bytes(total_size)).yellow()
    );

    if repositories.is_empty() {
        println!("{}", style("No repositories to export").yellow());
        return Ok(());
    }

    check_disk_space(&config.output_directory, total_size * 2).await?;

    if !is_sync 
        && !config.output_directory.join(&user.login).exists()
        && !Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!(
                "Export {} repositories to {}?",
                repositories.len(),
                config.output_directory.display()
            ))
            .default(true)
            .interact()?
    {
        println!("{}", style("Export cancelled").yellow());
        return Ok(());
    }

    println!("\n{}", style("Starting export...").bold());

    let progress = ProgressTracker::new(repositories.len());
    let downloader = Downloader::new(
        config.output_directory.clone(),
        config.github_token.unwrap(),
        config.shallow_clone,
        progress.clone(),
    );

    let results = downloader
        .download_repositories(repositories.clone(), config.parallel_downloads)
        .await?;
    progress.finish();

    let successful: Vec<_> = results
        .iter()
        .filter(|(_, result)| matches!(result, DownloadResult::Success))
        .collect();

    let failed: Vec<_> = results
        .iter()
        .filter(|(_, result)| matches!(result, DownloadResult::Failed(_)))
        .collect();

    println!("\n{}", style("Export Summary:").bold());
    println!("  Total: {}", repositories.len());
    println!("  Successful: {}", style(successful.len()).green());
    println!("  Failed: {}", style(failed.len()).red());

    if !failed.is_empty() {
        println!("\n{}", style("Failed repositories:").red());
        for (name, result) in &failed {
            if let DownloadResult::Failed(reason) = result {
                println!("  - {name}: {reason}");
            }
        }
    }

    let metadata = ExportMetadata {
        last_export: chrono::Utc::now(),
        total_repos: repositories.len(),
        successful_exports: successful.len(),
        failed_exports: failed.iter().map(|(name, _)| name.clone()).collect(),
        export_duration_seconds: start_time.elapsed().as_secs(),
    };

    let user_dir = config.output_directory.join(&user.login);
    metadata.save(&user_dir)?;

    let duration = chrono::Duration::seconds(start_time.elapsed().as_secs() as i64);
    println!(
        "\n{} Completed in {}",
        style("✓").green().bold(),
        style(format_duration(duration)).cyan()
    );

    Ok(())
}

fn merge_cli_config(config: &mut Config, cli: &Cli) {
    if let Some(token) = &cli.token {
        config.github_token = Some(token.clone());
    }

    if let Some(output) = &cli.output {
        config.output_directory = output.clone();
    }

    if cli.parallel > 0 {
        config.parallel_downloads = cli.parallel;
    }

    if cli.include_archived {
        config.include_archived = true;
    }

    if cli.exclude_forks {
        config.exclude_forks = true;
    }

    if cli.shallow {
        config.shallow_clone = true;
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}

fn format_duration(duration: chrono::Duration) -> String {
    let hours = duration.num_hours();
    let minutes = duration.num_minutes() % 60;
    let seconds = duration.num_seconds() % 60;

    if hours > 0 {
        format!("{hours}h {minutes}m {seconds}s")
    } else if minutes > 0 {
        format!("{minutes}m {seconds}s")
    } else {
        format!("{seconds}s")
    }
}
