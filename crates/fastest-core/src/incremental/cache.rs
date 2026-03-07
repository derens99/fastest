//! LRU result cache for incremental testing
//!
//! Stores previous test results so unchanged tests can be skipped.

use std::num::NonZeroUsize;

use lru::LruCache;

use crate::model::TestResult;

/// An LRU cache mapping test IDs to their most recent results.
///
/// When capacity is exceeded, the least recently used entry is evicted.
pub struct ResultCache {
    cache: LruCache<String, TestResult>,
}

impl ResultCache {
    /// Create a new cache with the given maximum capacity.
    ///
    /// A capacity of 0 is silently clamped to 1.
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity.max(1)).expect("capacity.max(1) is always >= 1");
        Self {
            cache: LruCache::new(cap),
        }
    }

    /// Look up a cached result by test ID, promoting it to most-recently-used.
    pub fn get(&mut self, test_id: &str) -> Option<&TestResult> {
        self.cache.get(test_id)
    }

    /// Insert (or update) a test result in the cache.
    pub fn insert(&mut self, test_id: String, result: TestResult) {
        self.cache.put(test_id, result);
    }

    /// Return the number of entries currently in the cache.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Return `true` if the cache contains no entries.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{TestOutcome, TestResult};
    use std::time::Duration;

    fn make_result(id: &str, outcome: TestOutcome) -> TestResult {
        TestResult {
            test_id: id.to_string(),
            outcome,
            duration: Duration::from_millis(10),
            output: String::new(),
            error: None,
            stdout: String::new(),
            stderr: String::new(),
        }
    }

    #[test]
    fn test_result_cache_basic() {
        let mut cache = ResultCache::new(10);
        assert!(cache.is_empty());

        let result = make_result("test_a", TestOutcome::Passed);
        cache.insert("test_a".to_string(), result);

        assert_eq!(cache.len(), 1);
        let cached = cache.get("test_a").unwrap();
        assert_eq!(cached.outcome, TestOutcome::Passed);

        // Missing key returns None
        assert!(cache.get("test_b").is_none());
    }

    #[test]
    fn test_result_cache_lru_eviction() {
        let mut cache = ResultCache::new(2);

        cache.insert("a".to_string(), make_result("a", TestOutcome::Passed));
        cache.insert("b".to_string(), make_result("b", TestOutcome::Failed));

        // Cache is full (capacity 2)
        assert_eq!(cache.len(), 2);

        // Inserting a third entry evicts the least-recently-used ("a")
        cache.insert("c".to_string(), make_result("c", TestOutcome::Passed));
        assert_eq!(cache.len(), 2);
        assert!(cache.get("a").is_none(), "a should have been evicted");
        assert!(cache.get("b").is_some());
        assert!(cache.get("c").is_some());
    }
}
