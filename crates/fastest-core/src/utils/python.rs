use once_cell::sync::Lazy;
use std::process::Command;

/// The detected Python command to use
pub static PYTHON_CMD: Lazy<String> =
    Lazy::new(|| detect_python_command().unwrap_or_else(|| "python3".to_string()));

/// Detect the correct Python command to use
pub fn detect_python_command() -> Option<String> {
    // Check for virtual environment first
    if let Ok(venv) = std::env::var("VIRTUAL_ENV") {
        // Try virtual environment's python
        let venv_python = std::path::Path::new(&venv).join("bin").join("python");
        if venv_python.exists() {
            if let Some(path_str) = venv_python.to_str() {
                if is_python_command_valid(path_str) {
                    eprintln!("Using virtual environment Python: {}", path_str);
                    return Some(path_str.to_string());
                }
            }
        }
    }

    // Check if we're in a conda environment
    if std::env::var("CONDA_DEFAULT_ENV").is_ok() && is_python_command_valid("python") {
        eprintln!("Using conda environment Python");
        return Some("python".to_string());
    }

    // Try python3 first (most common on modern systems)
    if is_python_command_valid("python3") {
        return Some("python3".to_string());
    }

    // Try python
    if is_python_command_valid("python") {
        return Some("python".to_string());
    }

    // Try to find Python in common locations
    let common_paths = [
        "/usr/bin/python3",
        "/usr/local/bin/python3",
        "/opt/homebrew/bin/python3",
        "C:\\Python39\\python.exe",
        "C:\\Python310\\python.exe",
        "C:\\Python311\\python.exe",
    ];

    for path in &common_paths {
        if is_python_command_valid(path) {
            return Some(path.to_string());
        }
    }

    None
}

/// Check if a Python command is valid
fn is_python_command_valid(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Get the Python version
pub fn get_python_version() -> Option<String> {
    Command::new(&*PYTHON_CMD)
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| {
            String::from_utf8(output.stdout)
                .or_else(|_| String::from_utf8(output.stderr))
                .ok()
        })
        .map(|v| v.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_detection() {
        assert!(!PYTHON_CMD.is_empty());
        println!("Detected Python command: {}", *PYTHON_CMD);
    }

    #[test]
    fn test_python_version() {
        if let Some(version) = get_python_version() {
            println!("Python version: {}", version);
            assert!(version.contains("Python"));
        }
    }
}
