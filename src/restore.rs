use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::models::{Pane, Window};

impl Window {
    fn from_line(line: &str) -> Result<Self, anyhow::Error> {
        // Format: # Window: session_name:index (name) active layout
        let line = line.trim_start_matches("# Window: ");
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 5 {
            anyhow::bail!("Failed to read window info from: {}", line);
        }

        let (session_name, index, name, active, layout) =
            (parts[0], parts[1], parts[2], parts[3], parts[4]);

        Ok(Window {
            session_name: session_name.to_string(),
            index: index
                .parse()
                .with_context(|| format!("Failed to parse window index: {}", index))?,
            name: name.to_string(),
            active: active == "1",
            layout: layout.to_string(),
            panes: Vec::new(),
        })
    }
}

pub fn restore_tmux_session(save_path: &PathBuf, session_name: &str, dry_run: bool) -> Result<()> {
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
            if let Ok(mut window) = Window::from_line(line) {
                // Collect all panes for this window until we hit the next window or EOF
                while let Some(next_line) = lines.peek() {
                    if next_line.starts_with("# Window: ") {
                        break;
                    }
                    if next_line.starts_with("# Pane: ") {
                        let pane_line = lines.next().unwrap();
                        // Skip the "# Pane: " prefix
                        let pane_data = &pane_line["# Pane: ".len()..];

                        let parts: Vec<&str> = pane_data.split('|').collect();

                        if parts.len() < 6 {
                            anyhow::bail!("Failed to read pane info from: {}", pane_line);
                        }

                        let (index, active, title, path, cmd, pid) =
                            (parts[0], parts[1], parts[2], parts[3], parts[4], parts[5]);
                        window.panes.push(Pane {
                            index: index.parse().unwrap_or(0),
                            active: active == "1",
                            title: title.to_string(),
                            current_path: path.to_string(),
                            current_command: cmd.to_string(),
                            pid: pid.parse().unwrap_or(0),
                        });
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

    // Clear all existing windows except the first one (tmux requires at least one window)
    Command::new("tmux")
        .args(["list-windows", "-t", session_name, "-F", "#{window_index}"])
        .output()
        .context("Failed to list existing windows")?
        .stdout
        .split(|&b| b == b'\n')
        .filter_map(|w| String::from_utf8_lossy(w).trim().parse::<u32>().ok())
        .filter(|&index| index != 0) // Keep the first window
        .try_for_each(|index| {
            Command::new("tmux")
                .args(["kill-window", "-t", &format!("{}:{}", session_name, index)])
                .output()
                .with_context(|| format!("Failed to kill window {}", index))
                .map(|_| ())
        })?;

    // Create remaining windows
    for window in windows.iter() {
        restore_window(session_name, window)?;
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

    Ok(())
}

fn restore_window(session_name: &str, window: &Window) -> Result<()> {
    if window.index > 0 {
        let window_format = format!("{}:{}", session_name, window.index);
        let mut args = vec!["new-window", "-t", &window_format, "-n", &window.name];

        let first_pane = window.panes.first();
        if let Some(pane) = first_pane {
            args.extend(["-c", &pane.current_path]);
        }

        Command::new("tmux")
            .args(&args)
            .output()
            .with_context(|| format!("Failed to create window {}", window.index))?;
    }

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

    Ok(())
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
