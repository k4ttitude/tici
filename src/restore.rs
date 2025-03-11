use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn restore_tmux_session(save_path: &PathBuf) -> Result<()> {
    // Check if file exists
    if !save_path.exists() {
        anyhow::bail!("No saved session found for this directory");
    }

    // Read the saved session file
    let content = fs::read_to_string(save_path).context("Failed to read saved session file")?;

    // Create a new session detached
    let session_name = "restored";
    Command::new("tmux")
        .args(["new-session", "-d", "-s", session_name])
        .output()
        .context("Failed to create new tmux session")?;

    let mut window_index = 0;
    let mut first_window = true;

    // Process each window section
    for section in content.split("---\n") {
        if section.trim().is_empty() {
            continue;
        }

        let lines: Vec<&str> = section.lines().collect();
        if lines.len() < 2 {
            continue;
        }

        // Parse window info and layout
        let layout_line = lines[1].trim_start_matches("# Layout: ");

        if !first_window {
            // Create new window for each section after the first
            Command::new("tmux")
                .args([
                    "new-window",
                    "-t",
                    &format!("{}:{}", session_name, window_index),
                ])
                .output()
                .with_context(|| format!("Failed to create window {}", window_index))?;
        }

        // Set the layout
        Command::new("tmux")
            .args([
                "select-layout",
                "-t",
                &format!("{}:{}", session_name, window_index),
                layout_line.trim(),
            ])
            .output()
            .with_context(|| format!("Failed to set layout for window {}", window_index))?;

        // Send the content to the pane
        let content_start = 2; // Skip window and layout lines
        let pane_content = lines[content_start..].join("\n");

        Command::new("tmux")
            .args([
                "send-keys",
                "-t",
                &format!("{}:{}", session_name, window_index),
                &pane_content,
                "Enter",
            ])
            .output()
            .with_context(|| format!("Failed to restore content for window {}", window_index))?;

        first_window = false;
        window_index += 1;
    }

    // Switch to the restored session
    Command::new("tmux")
        .args(["switch-client", "-t", session_name])
        .output()
        .context("Failed to switch to restored session")?;

    println!("Session restored successfully");
    Ok(())
}
