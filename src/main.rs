use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod models;
mod restore;
mod save;
mod session_info;
mod tmux;

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
    let (save_path, session_name) = session_info::get_session_info(cli.working_dir.as_ref())?;

    match &cli.command {
        Some(Commands::Save) => {
            if cli.dry_run {
                println!("Would save session to: {}", save_path.display());
            } else {
                save::save_tmux_session(&save_path)?;
            }
        }

        Some(Commands::Restore) => {
            restore::restore_tmux_session(&save_path, &session_name, cli.dry_run)
                .and(tmux::switch_to_session(&session_name))?;
        }

        None => {
            if cli.dry_run {
                println!("Would try to:");
                println!("1. Find and attach to session: {}", session_name);
                println!("2. Or create new session{}", session_name);
                println!("3. Then restore session from: {}\n", save_path.display());
                restore::restore_tmux_session(&save_path, &session_name, true)?;
                return Ok(());
            }

            // First try to find and attach to existing session
            if tmux::session_exists(&session_name)? {
                tmux::switch_to_session(&session_name)?;
                return Ok(());
            }

            // If no existing session, create a new one with -d (detached) option
            tmux::new_tmux_session(&session_name, true)
                .context(format!("Failed to create sesstion {}", session_name))?;

            // try restoring session, ignore errors (if any)
            let _ = restore::restore_tmux_session(&save_path, &session_name, false);

            // switch/attach to the session, then also save it
            tmux::switch_to_session(&session_name)?;
            save::save_tmux_session(&save_path)?;
        }
    }

    Ok(())
}
