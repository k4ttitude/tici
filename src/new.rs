use anyhow::{Context, Result};
use std::process::Command;

pub fn new_tmux_session(session_name: &str) -> Result<()> {
    let mut child = Command::new("tmux")
        .args(["new-session", "-s", session_name])
        .spawn()
        .context("Failed to create new session")?;

    // Wait for the tmux session to end
    child
        .wait()
        .context("Failed to wait for tmux session to complete")?;
    Ok(())
}
