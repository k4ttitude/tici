use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::env;
use std::path::PathBuf;

fn get_current_dir() -> Result<PathBuf> {
    env::current_dir().context("Failed to get current directory")
}

fn create_hash(path: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    format!("{:x}", hasher.finalize())[..16].to_string()
}

pub fn get_session_info(working_dir: Option<&PathBuf>) -> Result<(PathBuf, PathBuf, String)> {
    let dir = match working_dir {
        Some(path) => {
            let path = path.clone();
            if path.is_relative() {
                get_current_dir()?
                    .join(&path)
                    .canonicalize()
                    .context("Failed to resolve directory path")?
            } else {
                path.canonicalize()
                    .context("Failed to resolve directory path")?
            }
        }
        None => get_current_dir()?,
    };

    println!("Using directory: {}", dir.to_string_lossy());

    let dir_str = dir.to_string_lossy();
    let hash = create_hash(&dir_str);

    let session_name = dir
        .file_name()
        .and_then(|name| name.to_str())
        .context("Failed to get directory name")?
        .to_string();
    let filename = format!("session_{}_{}.tmux", hash, session_name);

    let home_dir = env::var("HOME").context("Failed to get HOME directory")?;
    let save_dir = PathBuf::from(home_dir).join(".tmux").join("tici");
    let save_path = save_dir.join(&filename);

    Ok((dir, save_path, session_name))
}
