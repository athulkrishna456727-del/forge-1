use std::fs;
use std::process::Command;

// Reads a file from disk and returns its contents as text
pub fn read_file(path: &str) -> anyhow::Result<String> {
    let content = fs::read_to_string(path)?;
    Ok(content)
}

// Writes text to a file, creating it if it doesn't exist
pub fn write_file(path: &str, content: &str) -> anyhow::Result<()> {
    fs::write(path, content)?;
    Ok(())
}

// Runs a shell command and returns its output
// WARNING: this is powerful — later we add safety checks (see Section 8)
const BLOCKED_PATTERNS: &[&str] = &["rm -rf", "del /f", "format ", "shutdown", ":(){ :|:& };:"];

pub fn run_command(cmd: &str) -> anyhow::Result<String> {
    let lower = cmd.to_lowercase();
    for pattern in BLOCKED_PATTERNS {
        if lower.contains(pattern) {
            return Ok(format!("BLOCKED: command contains a dangerous pattern ({})", pattern));
        }
    }

    let output = Command::new("cmd").args(["/C", cmd]).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok(format!("{}{}", stdout, stderr))
}