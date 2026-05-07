use clap::Parser;
use strand::cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            if let Err(e) = strand::commands::init::init() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Ls => {
            if let Err(e) = strand::commands::ls::execute() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::LsRemote => {
            if let Err(e) = strand::commands::ls_remote::execute() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Sync => {
            if let Err(e) = strand::commands::sync::execute() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Install { dry_run } => {
            if let Err(e) = strand::commands::install::execute(strand::commands::install::InstallOptions { dry_run }) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Validate => {
            if let Err(e) = strand::commands::validate::execute() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}
