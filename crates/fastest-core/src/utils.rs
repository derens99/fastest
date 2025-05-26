use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Get current timestamp in milliseconds
pub fn current_timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

/// Convert a path to a module name (e.g., path/to/test_file.py -> path.to.test_file)
pub fn path_to_module_name(path: &Path) -> String {
    path.to_str()
        .unwrap_or("")
        .replace('/', ".")
        .replace('\\', ".")
        .trim_end_matches(".py")
        .to_string()
}

/// Check if a path looks like a test file
pub fn is_test_file(path: &Path) -> bool {
    if let Some(name) = path.file_name() {
        let name = name.to_string_lossy();
        (name.starts_with("test_") || name.ends_with("_test.py")) && name.ends_with(".py")
    } else {
        false
    }
}

/// Get the project root by looking for common project markers
pub fn find_project_root(start: &Path) -> Option<PathBuf> {
    let markers = [
        "pyproject.toml",
        "setup.py",
        "setup.cfg",
        ".git",
        "tox.ini",
        "pytest.ini",
    ];

    let mut current = start;
    loop {
        for marker in &markers {
            if current.join(marker).exists() {
                return Some(current.to_path_buf());
            }
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => return None,
        }
    }
}

/// Format duration in human-readable format
pub fn format_duration(millis: u128) -> String {
    if millis < 1000 {
        format!("{}ms", millis)
    } else if millis < 60_000 {
        format!("{:.2}s", millis as f64 / 1000.0)
    } else {
        let minutes = millis / 60_000;
        let seconds = (millis % 60_000) / 1000;
        format!("{}m {}s", minutes, seconds)
    }
}
