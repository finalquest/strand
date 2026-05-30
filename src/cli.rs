use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "strand")]
#[command(about = "A CLI tool for managing GitLab skills")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new strand project
    Init,
    /// List installed skills with local vs remote version comparison
    Ls,
    /// List available skills from remote repository
    LsRemote,
    /// Sync installed skills with remote repository
    Sync,
    /// Install or reinstall skills from config
    Install {
        /// Show what would be installed without making changes
        #[arg(long)]
        dry_run: bool,
    },
    /// Validate skill structure and metadata
    Validate,
    /// Create a skill pack from local skills directory
    CreatePack,
    /// Manage agents
    Agents {
        #[command(subcommand)]
        command: AgentsCommands,
    },
}

#[derive(Subcommand)]
pub enum AgentsCommands {
    /// List installed agents with local vs remote version comparison
    Ls,
    /// List available agents from remote repository
    LsRemote,
    /// Validate agent structure and metadata
    Validate,
}
