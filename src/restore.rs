use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug)]
struct Pane {
    index: u32,
    title: String,
    current_path: String,
    active: bool,
    current_command: String,
    pid: u32,
    history_size: u32,
    content: String,
}

#[derive(Debug)]
struct Window {
    session_name: String,
    index: u32,
    name: String,
    active: bool,
    layout: String,
    panes: Vec<Pane>,
}

impl Window {
    fn from_line(line: &str) -> Option<Self> {
        // Format: # Window: session_name:index (name) active layout
        let line = line.trim_start_matches("# Window: ");
        let mut parts = line.split_whitespace();

        let session_index = parts.next()?;
        let (session_name, index) = session_index.split_once(':')?;

        let name = parts.next()?.trim_start_matches('(').trim_end_matches(')');
        let active = parts.next()? == "1";

        let layout = parts.next()?;

        Some(Window {
            session_name: session_name.to_string(),
            index: index.parse().ok()?,
            name: name.to_string(),
            active,
            layout: layout.to_string(),
            panes: Vec::new(),
        })
    }
}

pub fn restore_tmux_session(save_path: &PathBuf, dry_run: bool) -> Result<()> {
    // Check if file exists
    if !save_path.exists() {
        anyhow::bail!("No saved session found for this directory");
    }

    // Read the saved session file
    let content = fs::read_to_string(save_path).context("Failed to read saved session file")?;

    // Parse windows and panes from the content
    let mut windows: Vec<Window> = Vec::new();

    let mut lines = content.lines().peekable();
    while let Some(line) = lines.next() {
        if line.starts_with("# Window: ") {
            if let Some(mut window) = Window::from_line(line) {
                // Collect all panes for this window until we hit the next window or EOF
                while let Some(next_line) = lines.peek() {
                    if next_line.starts_with("# Window: ") {
                        break;
                    }
                    if next_line.starts_with("# Pane: ") {
                        let pane_line = lines.next().unwrap();
                        // Skip the "# Pane: " prefix
                        let pane_data = &pane_line["# Pane: ".len()..];

                        // Split on first 7 spaces to preserve any spaces in the remaining fields
                        let mut splits = pane_data.splitn(7, ' ');

                        if let (
                            Some(index),
                            Some(active),
                            Some(title),
                            Some(path),
                            Some(cmd),
                            Some(pid),
                            Some(history),
                        ) = (
                            splits.next(),
                            splits.next(),
                            splits.next(),
                            splits.next(),
                            splits.next(),
                            splits.next(),
                            splits.next(),
                        ) {
                            window.panes.push(Pane {
                                index: index.parse().unwrap_or(0),
                                active: active == "1",
                                title: title.to_string(),
                                current_path: path.to_string(),
                                current_command: cmd.to_string(),
                                pid: pid.parse().unwrap_or(0),
                                history_size: history.parse().unwrap_or(0),
                                content: String::new(),
                            });
                        }
                    } else {
                        lines.next(); // Skip non-pane lines
                    }
                }
                windows.push(window);
            }
        }
    }

    if windows.is_empty() {
        anyhow::bail!("No windows found in saved session");
    }

    if dry_run {
        print_session_info(&windows);
        return Ok(());
    }

    // Create a new session with the first window
    let first_window = &windows[0];
    let session_name = &first_window.session_name;

    Command::new("tmux")
        .args([
            "new-session",
            "-d",
            "-s",
            session_name,
            "-n",
            &first_window.name,
        ])
        .output()
        .context("Failed to create new tmux session")?;

    // Create additional panes for the first window
    for pane in first_window.panes.iter().skip(1) {
        Command::new("tmux")
            .args([
                "split-window",
                "-t",
                &format!("{}:0", session_name),
                "-c",
                &pane.current_path,
            ])
            .output()
            .with_context(|| format!("Failed to create pane {} in window 0", pane.index))?;
    }

    // Set the layout for the first window
    Command::new("tmux")
        .args([
            "select-layout",
            "-t",
            &format!("{}:0", session_name),
            &first_window.layout,
        ])
        .output()
        .context("Failed to set layout for first window")?;

    // Create remaining windows
    for window in windows.iter().skip(1) {
        Command::new("tmux")
            .args([
                "new-window",
                "-t",
                &format!("{}:{}", session_name, window.index),
                "-n",
                &window.name,
            ])
            .output()
            .with_context(|| format!("Failed to create window {}", window.index))?;

        // Create additional panes (skip first pane as it's created with new-window)
        for pane in window.panes.iter().skip(1) {
            Command::new("tmux")
                .args([
                    "split-window",
                    "-t",
                    &format!("{}:{}", session_name, window.index),
                    "-c",
                    &pane.current_path,
                ])
                .output()
                .with_context(|| {
                    format!(
                        "Failed to create pane {} in window {}",
                        pane.index, window.index
                    )
                })?;
        }

        // Set the layout
        Command::new("tmux")
            .args([
                "select-layout",
                "-t",
                &format!("{}:{}", session_name, window.index),
                &window.layout,
            ])
            .output()
            .with_context(|| format!("Failed to set layout for window {}", window.index))?;

        // Restore pane contents
        for pane in &window.panes {
            // If this is the active pane, select it
            let target = format!("{}:{}.{}", session_name, window.index, pane.index);
            if pane.active {
                Command::new("tmux")
                    .args(["select-pane", "-t", &target])
                    .output()
                    .with_context(|| format!("Failed to select active pane {}", pane.index))?;
            }
        }
    }

    // Select the active window if any
    if let Some(active_window) = windows.iter().find(|w| w.active) {
        Command::new("tmux")
            .args([
                "select-window",
                "-t",
                &format!("{}:{}", session_name, active_window.index),
            ])
            .output()
            .context("Failed to select active window")?;
    }

    // Either switch to or attach to the restored session based on whether we're in tmux
    if is_inside_tmux() {
        Command::new("tmux")
            .args(["switch-client", "-t", session_name])
            .output()
            .context("Failed to switch to restored session")?;
        println!("Session restored successfully");
    } else {
        // For attach-session, we need to use spawn() and wait() instead of output()
        // because attach needs to take control of the terminal
        let mut child = Command::new("tmux")
            .args(["attach-session", "-t", session_name])
            .spawn()
            .context("Failed to attach to restored session")?;

        // Wait for the tmux session to end
        child
            .wait()
            .context("Failed to wait for tmux session to complete")?;
    }
    Ok(())
}

fn is_inside_tmux() -> bool {
    std::env::var("TMUX").is_ok()
}

fn print_session_info(windows: &[Window]) {
    println!("Session: {}", windows[0].session_name);
    for window in windows {
        println!(
            "\nWindow {} ({}){}",
            window.index,
            window.name,
            if window.active { " [active]" } else { "" }
        );
        println!("Layout: {}", window.layout);

        for pane in &window.panes {
            println!(
                "  Pane {}{}:",
                pane.index,
                if pane.active { " [active]" } else { "" }
            );
            println!("    Title: {}", pane.title);
            println!("    Path: {}", pane.current_path);
            println!("    Command: {}", pane.current_command);
            println!("    PID: {}", pane.pid);
        }
    }
}
