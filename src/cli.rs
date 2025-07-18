use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "gh-export")]
#[command(author, version, about = "Export all GitHub repositories from a user account", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(
        short,
        long,
        env = "GITHUB_TOKEN",
        help = "GitHub personal access token"
    )]
    pub token: Option<String>,

    #[arg(short, long, help = "Output directory for repositories")]
    pub output: Option<PathBuf>,

    #[arg(
        short,
        long,
        default_value = "4",
        help = "Number of parallel downloads"
    )]
    pub parallel: usize,

    #[arg(long, help = "Include archived repositories")]
    pub include_archived: bool,

    #[arg(long, help = "Exclude forked repositories")]
    pub exclude_forks: bool,

    #[arg(long, help = "Perform shallow clones (depth=1)")]
    pub shallow: bool,

    #[arg(short, long, help = "Filter repositories by name pattern")]
    pub filter: Option<String>,

    #[arg(short, long, help = "Quiet mode - minimal output")]
    pub quiet: bool,

    #[arg(short, long, help = "Verbose logging")]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Re-sync existing repositories")]
    Sync {
        #[arg(
            short,
            long,
            help = "Only update repositories modified after this date (YYYY-MM-DD)"
        )]
        since: Option<String>,
    },

    #[command(about = "Manage stored configuration")]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    #[command(about = "Show information about the last export")]
    Status,
}

#[derive(Subcommand, Clone)]
pub enum ConfigAction {
    #[command(about = "Show current configuration")]
    Show,

    #[command(about = "Set a configuration value")]
    Set {
        #[arg(help = "Configuration key (token, output, parallel)")]
        key: String,

        #[arg(help = "Value to set")]
        value: String,
    },

    #[command(about = "Clear stored configuration")]
    Clear,
}
