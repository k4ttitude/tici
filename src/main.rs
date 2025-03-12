use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use sha2::{Digest, Sha256};
use std::env;
use std::path::PathBuf;

mod restore;
mod save;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Save the current tmux session
    Save,
    /// Restore the tmux session for current directory
    Restore {
        /// Dry run - only print session information without creating it
        #[arg(short = 'n')]
        dry_run: bool,
    },
}

fn get_current_dir() -> Result<PathBuf> {
    env::current_dir().context("Failed to get current directory")
}

fn create_hash(path: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    format!("{:x}", hasher.finalize())[..16].to_string()
}

fn get_session_file() -> Result<(PathBuf, String)> {
    let current_dir = get_current_dir()?;
    let dir_str = current_dir.to_string_lossy();
    let hash = create_hash(&dir_str);
    let filename = format!("session_{}.tmux", hash);

    let home_dir = env::var("HOME").context("Failed to get HOME directory")?;
    let save_dir = PathBuf::from(home_dir).join(".tmux").join("tici");
    let save_path = save_dir.join(&filename);

    Ok((save_path, filename))
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let (save_path, _) = get_session_file()?;

    match cli.command.unwrap_or(Commands::Restore { dry_run: false }) {
        Commands::Save => {
            save::save_tmux_session(&save_path)?;
        }
        Commands::Restore { dry_run } => {
            restore::restore_tmux_session(&save_path, dry_run)?;
        }
    }

    Ok(())
}
