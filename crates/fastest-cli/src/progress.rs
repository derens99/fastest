//! Progress bar support for test execution.
//!
//! Uses the `indicatif` crate to display a live progress bar during test runs.

use indicatif::{ProgressBar, ProgressStyle};

/// Create a styled progress bar for tracking test execution.
///
/// The bar displays: `[===>   ] 42/100 tests   test_math::test_add`
pub fn create_progress_bar(total: usize) -> ProgressBar {
    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} tests  {msg}",
        )
        .unwrap_or_else(|_| ProgressStyle::default_bar())
        .progress_chars("=>-"),
    );
    pb.set_message("running...");
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
    fn test_create_progress_bar_zero() {
        let pb = create_progress_bar(0);
        assert_eq!(pb.length(), Some(0));
    }
}
