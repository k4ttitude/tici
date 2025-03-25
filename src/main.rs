use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod new;
mod restore;
mod save;
mod session;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Optional working directory to use for the session
    #[arg(short = 'd', long = "dir", global = true)]
    working_dir: Option<PathBuf>,

    /// Dry run - only print information without making changes
    #[arg(short = 'n', long = "dry-run", global = true)]
    dry_run: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Save the current tmux session
    Save,

    /// Restore the tmux session for the specified directory
    Restore,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let (save_path, session_name) = session::get_session_info(cli.working_dir.as_ref())?;

    match &cli.command {
        Some(Commands::Save) => {
            if cli.dry_run {
                println!("Would save session to: {}", save_path.display());
            } else {
                save::save_tmux_session(&save_path)?;
            }
        }
        Some(Commands::Restore) => {
            restore::restore_tmux_session(&save_path, cli.dry_run)?;
        }
        None => {
            // Default behavior: try to restore, or create new if no session exists
            restore::restore_tmux_session(&save_path, cli.dry_run).or_else(|_| {
                if cli.dry_run {
                    println!("Would create new session: {}", session_name);
                    Ok(())
                } else {
                    new::new_tmux_session(&session_name)
                }
            })?;
        }
    }

    Ok(())
}
