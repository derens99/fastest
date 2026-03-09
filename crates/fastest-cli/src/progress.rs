//! Progress display for test execution.
//!
//! Uses the `indicatif` crate to show a progress bar as tests complete.

use indicatif::{ProgressBar, ProgressStyle};

/// Create a progress bar for test execution.
///
/// Displays a progress bar showing `[15/100] tests running... ..F.s.`
/// that updates as each test result arrives via the streaming callback.
pub fn create_progress_bar(total: usize) -> ProgressBar {
    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} [{pos}/{len}] {msg} [{elapsed_precise}]")
            .unwrap_or_else(|_| ProgressStyle::default_bar()),
    );
    pb.set_message("running tests...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb
}

/// Create a simple spinner for when total count is unknown.
#[cfg(test)]
fn create_spinner(total: usize) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} running {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.set_message(format!("{} tests...", total));
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_progress_bar() {
        let pb = create_progress_bar(100);
        assert_eq!(pb.length(), Some(100));
    }

    #[test]
    fn test_create_spinner() {
        let pb = create_spinner(100);
        // Spinner has no length
        assert_eq!(pb.length(), None);
    }

    #[test]
    fn test_create_spinner_zero() {
        let pb = create_spinner(0);
        assert_eq!(pb.length(), None);
    }
}
