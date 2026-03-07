//! Progress display for test execution.
//!
//! Uses the `indicatif` crate to show a spinner while tests run.

use indicatif::{ProgressBar, ProgressStyle};

/// Create a spinner for test execution.
///
/// Displays a spinning indicator with the test count while the executor runs.
/// This is preferable to a progress bar because the hybrid executor returns
/// all results at once rather than streaming them.
pub fn create_spinner(total: usize) -> ProgressBar {
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
