use anyhow::{Context, Result};
use std::process::Command;

fn is_inside_tmux() -> bool {
    std::env::var("TMUX").is_ok()
}

pub fn session_exists(session_name: &str) -> Result<bool> {
    let output = Command::new("tmux")
        .args(["has-session", "-t", session_name])
        .output()?;

    Ok(output.status.success())
}

pub fn switch_to_session(session_name: &str) -> Result<()> {
    if !is_inside_tmux() {
        let mut child = Command::new("tmux")
            .args(["attach-session", "-t", session_name])
            .spawn()?;

        let status = child.wait()?;

        if !status.success() {
            anyhow::bail!("Failed to attach to tmux session: {}", session_name);
        }

        return Ok(());
    }

    Command::new("tmux")
        .args(["switch-client", "-t", session_name])
        .output()
        .context("Failed to switch to tmux session")?;
    Ok(())
}

pub fn new_tmux_session(session_name: &str, detached: bool) -> Result<()> {
    let mut args = vec!["new-session", "-s", session_name];

    if detached {
        args.push("-d");
        Command::new("tmux")
            .args(args)
            .output()
            .context("Failed to create new session")?;
    } else {
        let mut child = Command::new("tmux").args(&args).spawn()?;

        let status = child.wait()?;
        if !status.success() {
            anyhow::bail!("Failed to create tmux session: {}", session_name);
        }
    }

    Ok(())
}
