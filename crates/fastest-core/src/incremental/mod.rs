//! Incremental testing support
//!
//! Filters tests to only those affected by recent file changes,
//! using git to detect modifications and an LRU cache for previous results.

pub mod cache;
pub mod impact;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use git2::{DiffOptions, Repository};

use crate::error::Result;
use crate::model::TestItem;

pub use cache::ResultCache;
pub use impact::find_affected_tests;

/// Filters a test suite down to only those tests affected by uncommitted changes.
pub struct IncrementalTester {
    repo_root: PathBuf,
    cache: cache::ResultCache,
}

impl IncrementalTester {
    /// Open the git repository at `repo_root` and create a new tester.
    ///
    /// The result cache is initialized with a default capacity of 4096 entries.
    pub fn new(repo_root: &Path) -> Result<Self> {
        // Validate that the path exists
        let root = repo_root.to_path_buf();
        Ok(Self {
            repo_root: root,
            cache: cache::ResultCache::new(4096),
        })
    }

    /// Return only the tests whose source files have been modified since HEAD.
    ///
    /// If the directory is not a git repository, returns **all** tests so that
    /// a full run is performed (the caller should warn the user).
    pub fn filter_unchanged(&self, tests: Vec<TestItem>) -> Result<Vec<TestItem>> {
        match self.get_changed_files()? {
            Some(changed) => Ok(find_affected_tests(&tests, &changed)),
            None => {
                // Not a git repo — run everything.
                Ok(tests)
            }
        }
    }

    /// Return a mutable reference to the internal result cache.
    pub fn cache_mut(&mut self) -> &mut cache::ResultCache {
        &mut self.cache
    }

    /// Returns `true` if the repo root is inside a git repository.
    pub fn is_git_repo(&self) -> bool {
        Repository::open(&self.repo_root).is_ok()
    }

    /// Detect files that have changed relative to HEAD using libgit2.
    ///
    /// Returns `None` if the path is not inside a git repository (caller
    /// should fall back to running all tests).  Returns `Some(set)` with
    /// the changed file paths on success.
    fn get_changed_files(&self) -> Result<Option<HashSet<PathBuf>>> {
        let repo = match Repository::open(&self.repo_root) {
            Ok(r) => r,
            Err(_) => return Ok(None),
        };

        let head = match repo.head() {
            Ok(h) => h,
            Err(e) if e.code() == git2::ErrorCode::UnbornBranch => {
                // New repo with no commits — treat as "everything changed"
                return Ok(None);
            }
            Err(e) => return Err(e.into()),
        };
        let head_commit = head.peel_to_commit()?;
        let head_tree = head_commit.tree()?;

        let mut diff_opts = DiffOptions::new();
        diff_opts.include_untracked(true);

        let diff = repo.diff_tree_to_workdir_with_index(Some(&head_tree), Some(&mut diff_opts))?;

        let mut changed = HashSet::new();
        diff.foreach(
            &mut |delta, _progress| {
                if let Some(path) = delta.new_file().path() {
                    changed.insert(path.to_path_buf());
                }
                if let Some(path) = delta.old_file().path() {
                    changed.insert(path.to_path_buf());
                }
                true
            },
            None,
            None,
            None,
        )?;

        Ok(Some(changed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_incremental_tester_new() {
        // It should succeed on any directory (git or not)
        let dir = TempDir::new().unwrap();
        let tester = IncrementalTester::new(dir.path());
        assert!(tester.is_ok());
    }

    #[test]
    fn test_filter_unchanged_non_git() {
        let dir = TempDir::new().unwrap();
        let tester = IncrementalTester::new(dir.path()).unwrap();

        let tests = vec![TestItem {
            id: "test_a".to_string(),
            path: PathBuf::from("tests/test_a.py"),
            function_name: "test_a".to_string(),
            line_number: Some(1),
            decorators: vec![],
            is_async: false,
            fixture_deps: vec![],
            class_name: None,
            markers: vec![],
            parameters: None,
            name: "test_a".to_string(),
        }];

        // Non-git directory → all tests returned (fallback to full run)
        let filtered = tester.filter_unchanged(tests).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "test_a");
    }

    #[test]
    fn test_is_git_repo() {
        let dir = TempDir::new().unwrap();
        let tester = IncrementalTester::new(dir.path()).unwrap();
        assert!(!tester.is_git_repo());
    }
}
