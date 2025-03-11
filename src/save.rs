use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn save_tmux_session(filename: &str) -> Result<()> {
    // Create the .tmux-here directory if it doesn't exist
    let home_dir = env::var("HOME").context("Failed to get HOME directory")?;
    let save_dir = PathBuf::from(home_dir).join(".tmux-here");
    fs::create_dir_all(&save_dir).context("Failed to create .tmux-here directory")?;

    let save_path = save_dir.join(filename);

    // Save the current tmux session
    let output = Command::new("tmux")
        .args(["list-windows", "-F", "#{session_name}:#{window_index}"])
        .output()
        .context("Failed to execute tmux list-windows")?;

    if !output.status.success() {
        anyhow::bail!("Failed to list tmux windows");
    }

    let session_info = String::from_utf8(output.stdout).context("Failed to parse tmux output")?;

    // Save the session layout to file
    let mut content = String::new();

    for line in session_info.lines() {
        let capture = Command::new("tmux")
            .args(["capture-pane", "-p", "-t", line])
            .output()
            .with_context(|| format!("Failed to capture pane content for {}", line))?;

        let layout = Command::new("tmux")
            .args(["list-panes", "-t", line, "-F", "#{window_layout}"])
            .output()
            .with_context(|| format!("Failed to get layout for {}", line))?;

        if capture.status.success() && layout.status.success() {
            let pane_content =
                String::from_utf8(capture.stdout).context("Failed to parse pane content")?;
            let layout_str = String::from_utf8(layout.stdout).context("Failed to parse layout")?;

            content.push_str(&format!("# Window: {}\n", line));
            content.push_str(&format!("# Layout: {}", layout_str));
            content.push_str(&pane_content);
            content.push_str("\n---\n");
        }
    }

    fs::write(&save_path, content)
        .with_context(|| format!("Failed to write to file: {}", save_path.display()))?;

    println!("Session saved to: {}", save_path.display());
    Ok(())
}
