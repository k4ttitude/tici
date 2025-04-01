use anyhow::{Context, Result};
use std::process::Command;

pub fn is_inside_tmux() -> bool {
    std::env::var("TMUX").is_ok()
}

fn get_current_session() -> Result<String> {
    let output = Command::new("tmux")
        .args(["display-message", "-p", "#{session_name}"])
        .output()
        .context("Failed to get current tmux session name")?;

    Ok(String::from_utf8(output.stdout)
        .context("Failed to parse tmux session name")?
        .trim()
        .to_string())
}

pub fn session_exists(session_name: &str) -> Result<bool> {
    let output = Command::new("tmux")
        .args(["has-session", "-t", session_name])
        .output()?;

    Ok(output.status.success())
}

pub fn switch_to_session(session_name: &str) -> Result<()> {
    let args = if is_inside_tmux() {
        ["switch-client", "-t", session_name]
    } else {
        ["attach-session", "-t", session_name]
    };

    let mut child = Command::new("tmux").args(args).spawn()?;

    let status = child.wait()?;
    if !status.success() {
        anyhow::bail!("Failed to switch to tmux session: {}", session_name);
    }

    Ok(())
}

#[derive(Default)]
pub struct NewSessionOpts {
    pub detached: bool,
    pub path: Option<String>,
}

pub fn new_tmux_session(session_name: &str, opts: NewSessionOpts) -> Result<()> {
    let mut args = vec!["new-session", "-s", session_name];

    if opts.detached {
        args.push("-d");
    }

    if let Some(path) = &opts.path {
        if !path.is_empty() {
            args.extend(["-c", path]);
        }
    }

    if opts.detached {
        Command::new("tmux")
            .args(&args)
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
