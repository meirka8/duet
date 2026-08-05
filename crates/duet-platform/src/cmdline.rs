use duet_types::{VPath, VfsError, VfsResult};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Output result from command line shell execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildProcessOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Run a command line string in user's shell with cwd specified.
pub fn run_shell_command(command: &str, cwd: &Path) -> VfsResult<ChildProcessOutput> {
    crate::assert_not_ui_thread();

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let output = Command::new(&shell)
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .output()
        .map_err(|e| VfsError::Fatal(format!("Shell execution error: {e}")))?;

    Ok(ChildProcessOutput {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

/// History file store for command line persistence (`~/.local/state/duet/history/cmdline.txt`).
#[derive(Debug, Default)]
pub struct HistoryStore;

impl HistoryStore {
    pub fn new() -> Self {
        Self
    }

    /// Load history commands from file path.
    pub fn load_history(&self, path: &Path) -> VfsResult<Vec<String>> {
        if !path.exists() {
            return Ok(Vec::new());
        }
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut history = Vec::new();
        for line in reader.lines() {
            let l = line?;
            if !l.trim().is_empty() {
                history.push(l);
            }
        }
        Ok(history)
    }

    /// Append command entry to history file.
    pub fn append_history(&self, path: &Path, command: &str) -> VfsResult<()> {
        if command.trim().is_empty() {
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        writeln!(file, "{command}")?;
        file.flush()?;
        Ok(())
    }

    /// Default history file path (`~/.local/state/duet/history/cmdline.txt`).
    pub fn default_history_path() -> PathBuf {
        if let Some(home) = std::env::var_os("HOME") {
            PathBuf::from(home)
                .join(".local")
                .join("state")
                .join("duet")
                .join("history")
                .join("cmdline.txt")
        } else {
            PathBuf::from("/tmp/duet_cmdline_history.txt")
        }
    }
}

/// Escape special characters in file paths for shell safety.
pub fn escape_shell_path(path: &str) -> String {
    if path.chars().all(|c| c.is_alphanumeric() || c == '/' || c == '.' || c == '_' || c == '-') {
        path.to_string()
    } else {
        format!("'{}'", path.replace('\'', "'\"'\"'"))
    }
}

/// Format and insert filename helper for command line.
pub fn format_insert_name(vpath: &VPath) -> String {
    let name = vpath.file_name().unwrap_or("");
    escape_shell_path(name)
}

/// Format and insert full path helper for command line.
pub fn format_insert_path(vpath: &VPath) -> String {
    escape_shell_path(&vpath.path)
}
