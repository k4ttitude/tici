use anyhow::{Context, Result};
use std::env;
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
    fn from_format_str(format_str: &str) -> Option<Self> {
        let parts: Vec<&str> = format_str.split('\t').collect();
        if parts.len() >= 5 {
            Some(Window {
                session_name: parts[1].to_string(),
                index: parts[2].parse().ok()?,
                name: parts[3].trim_start_matches(':').to_string(),
                active: parts[4] == "1",
                layout: parts[5].to_string(),
                panes: Vec::new(),
            })
        } else {
            None
        }
    }

    fn get_panes(&mut self) -> Result<()> {
        let target = format!("{}:{}", self.session_name, self.index);

        // Get list of panes for this window
        let pane_list = Command::new("tmux")
            .args([
                "list-panes",
                "-t",
                &target,
                "-F",
                "pane\t#{session_name}\t#{window_index}\t#{window_active}\t:#{window_flags}\t#{pane_index}\t#{pane_title}\t:#{pane_current_path}\t#{pane_active}\t#{pane_current_command}\t#{pane_pid}\t#{history_size}",
            ])
            .output()
            .with_context(|| format!("Failed to list panes for window {}", self.index))?;

        if !pane_list.status.success() {
            return Ok(());
        }

        let pane_list = String::from_utf8(pane_list.stdout)?;

        for pane_info in pane_list.lines() {
            let parts: Vec<&str> = pane_info.split('\t').collect();
            if parts.len() >= 12 {
                let index = parts[5].parse().unwrap_or(0);
                let title = parts[6].to_string();
                let current_path = parts[7].trim_start_matches(':').to_string();
                let active = parts[8] == "1";
                let current_command = parts[9].to_string();
                let pid = parts[10].parse().unwrap_or(0);
                let history_size = parts[11].parse().unwrap_or(0);

                // Capture pane content
                let content = Command::new("tmux")
                    .args([
                        "capture-pane",
                        "-p",
                        "-t",
                        &format!("{}:{}.{}", self.session_name, self.index, index),
                    ])
                    .output()
                    .with_context(|| {
                        format!(
                            "Failed to capture content for pane {}.{}",
                            self.index, index
                        )
                    })?;

                if content.status.success() {
                    self.panes.push(Pane {
                        index,
                        title,
                        current_path,
                        active,
                        current_command,
                        pid,
                        history_size,
                    });
                }
            }
        }

        Ok(())
    }
}

pub fn save_tmux_session(filename: &str) -> Result<()> {
    // Create the .tmux-here directory if it doesn't exist
    let home_dir = env::var("HOME").context("Failed to get HOME directory")?;
    let save_dir = PathBuf::from(home_dir).join(".tmux-here");
    fs::create_dir_all(&save_dir).context("Failed to create .tmux-here directory")?;

    let save_path = save_dir.join(filename);

    // Save the current tmux session with the specified format
    let format = "window\t#{session_name}\t#{window_index}\t:#{window_name}\t#{window_active}\t#{window_layout}";
    let output = Command::new("tmux")
        .args(["list-windows", "-F", format])
        .output()
        .context("Failed to execute tmux list-windows")?;

    if !output.status.success() {
        anyhow::bail!("Failed to list tmux windows");
    }

    let session_info = String::from_utf8(output.stdout).context("Failed to parse tmux output")?;

    // Save the session layout to file
    let mut content = String::new();

    for line in session_info.lines() {
        let mut window = Window::from_format_str(line)
            .with_context(|| format!("Failed to parse window format: {}", line))?;

        content.push_str(&format!(
            "# Window: {}:{} ({}) {} {}\n",
            window.session_name,
            window.index,
            window.name,
            if window.active { "1" } else { "0" },
            window.layout
        ));

        window.get_panes()?;

        for pane in &window.panes {
            content.push_str(&format!(
                "# Pane: {} {} {} {} {} {} {}\n",
                pane.index,
                if pane.active { "1" } else { "0" },
                pane.title,
                pane.current_path,
                pane.current_command,
                pane.pid,
                pane.history_size
            ));
        }
    }

    fs::write(&save_path, content)
        .with_context(|| format!("Failed to write to file: {}", save_path.display()))?;

    println!("Session saved to: {}", save_path.display());
    Ok(())
}
