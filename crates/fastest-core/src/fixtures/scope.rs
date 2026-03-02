//! Scope-aware fixture value caching.
//!
//! Maintains separate caches for each fixture scope so that values can be
//! shared and invalidated at the correct granularity.

use crate::fixtures::FixtureScope;
use std::collections::HashMap;

/// A scope-aware cache for fixture return values.
///
/// Fixture values are cached per scope so that, for example, a session-scoped
/// fixture is computed once and reused across the entire test session, while a
/// function-scoped fixture is recomputed for every test.
pub struct FixtureCache {
    function_cache: HashMap<String, serde_json::Value>,
    class_cache: HashMap<String, serde_json::Value>,
    module_cache: HashMap<String, serde_json::Value>,
    package_cache: HashMap<String, serde_json::Value>,
    session_cache: HashMap<String, serde_json::Value>,
}

impl FixtureCache {
    /// Create a new, empty fixture cache.
    pub fn new() -> Self {
        FixtureCache {
            function_cache: HashMap::new(),
            class_cache: HashMap::new(),
            module_cache: HashMap::new(),
            package_cache: HashMap::new(),
            session_cache: HashMap::new(),
        }
    }

    /// Look up a cached fixture value by name and scope.
    pub fn get(&self, name: &str, scope: &FixtureScope) -> Option<&serde_json::Value> {
        self.cache_for_scope(scope).get(name)
    }

    /// Insert a fixture value into the cache at the given scope.
    pub fn insert(&mut self, name: String, scope: &FixtureScope, value: serde_json::Value) {
        self.cache_for_scope_mut(scope).insert(name, value);
    }

    /// Clear all cached values for a given scope.
    ///
    /// This is called at scope boundaries -- for example, `clear_scope(Function)`
    /// is called between every test, while `clear_scope(Module)` is called when
    /// switching to a new test module.
    pub fn clear_scope(&mut self, scope: &FixtureScope) {
        self.cache_for_scope_mut(scope).clear();
    }

    /// Return the number of cached entries for a given scope.
    pub fn scope_len(&self, scope: &FixtureScope) -> usize {
        self.cache_for_scope(scope).len()
    }

    /// Return an immutable reference to the cache map for a given scope.
    fn cache_for_scope(&self, scope: &FixtureScope) -> &HashMap<String, serde_json::Value> {
        match scope {
            FixtureScope::Function => &self.function_cache,
            FixtureScope::Class => &self.class_cache,
            FixtureScope::Module => &self.module_cache,
            FixtureScope::Package => &self.package_cache,
            FixtureScope::Session => &self.session_cache,
        }
    }

    /// Return a mutable reference to the cache map for a given scope.
    fn cache_for_scope_mut(
        &mut self,
        scope: &FixtureScope,
    ) -> &mut HashMap<String, serde_json::Value> {
        match scope {
            FixtureScope::Function => &mut self.function_cache,
            FixtureScope::Class => &mut self.class_cache,
            FixtureScope::Module => &mut self.module_cache,
            FixtureScope::Package => &mut self.package_cache,
            FixtureScope::Session => &mut self.session_cache,
        }
    }
}

impl Default for FixtureCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_cache_isolation() {
        let mut cache = FixtureCache::new();

        // Insert the same key into different scopes with different values
        cache.insert(
            "db".to_string(),
            &FixtureScope::Function,
            serde_json::json!("func_db"),
        );
        cache.insert(
            "db".to_string(),
            &FixtureScope::Session,
            serde_json::json!("session_db"),
        );
        cache.insert(
            "db".to_string(),
            &FixtureScope::Module,
            serde_json::json!("module_db"),
        );

        // Each scope should return its own value
        assert_eq!(
            cache.get("db", &FixtureScope::Function),
            Some(&serde_json::json!("func_db")),
        );
        assert_eq!(
            cache.get("db", &FixtureScope::Session),
            Some(&serde_json::json!("session_db")),
        );
        assert_eq!(
            cache.get("db", &FixtureScope::Module),
            Some(&serde_json::json!("module_db")),
        );

        // Class and package scopes should have nothing
        assert_eq!(cache.get("db", &FixtureScope::Class), None);
        assert_eq!(cache.get("db", &FixtureScope::Package), None);
    }

    #[test]
    fn test_clear_scope() {
        let mut cache = FixtureCache::new();

        cache.insert(
            "fix_a".to_string(),
            &FixtureScope::Function,
            serde_json::json!(1),
        );
        cache.insert(
            "fix_b".to_string(),
            &FixtureScope::Function,
            serde_json::json!(2),
        );
        cache.insert(
            "fix_c".to_string(),
            &FixtureScope::Session,
            serde_json::json!(3),
        );

        assert_eq!(cache.scope_len(&FixtureScope::Function), 2);
        assert_eq!(cache.scope_len(&FixtureScope::Session), 1);

        // Clearing function scope should not affect session scope
        cache.clear_scope(&FixtureScope::Function);

        assert_eq!(cache.scope_len(&FixtureScope::Function), 0);
        assert_eq!(cache.get("fix_a", &FixtureScope::Function), None);
        assert_eq!(cache.get("fix_b", &FixtureScope::Function), None);

        // Session cache should be untouched
        assert_eq!(cache.scope_len(&FixtureScope::Session), 1);
        assert_eq!(
            cache.get("fix_c", &FixtureScope::Session),
            Some(&serde_json::json!(3)),
        );
    }

    #[test]
    fn test_insert_overwrites() {
        let mut cache = FixtureCache::new();

        cache.insert(
            "x".to_string(),
            &FixtureScope::Module,
            serde_json::json!("old"),
        );
        cache.insert(
            "x".to_string(),
            &FixtureScope::Module,
            serde_json::json!("new"),
        );

        assert_eq!(
            cache.get("x", &FixtureScope::Module),
            Some(&serde_json::json!("new")),
        );
        assert_eq!(cache.scope_len(&FixtureScope::Module), 1);
    }

    #[test]
    fn test_get_missing_returns_none() {
        let cache = FixtureCache::new();
        assert_eq!(cache.get("nonexistent", &FixtureScope::Function), None);
    }

    #[test]
    fn test_default_impl() {
        let cache = FixtureCache::default();
        assert_eq!(cache.scope_len(&FixtureScope::Function), 0);
        assert_eq!(cache.scope_len(&FixtureScope::Class), 0);
        assert_eq!(cache.scope_len(&FixtureScope::Module), 0);
        assert_eq!(cache.scope_len(&FixtureScope::Package), 0);
        assert_eq!(cache.scope_len(&FixtureScope::Session), 0);
    }

    #[test]
    fn test_clear_all_scopes_independently() {
        let mut cache = FixtureCache::new();

        let scopes = [
            FixtureScope::Function,
            FixtureScope::Class,
            FixtureScope::Module,
            FixtureScope::Package,
            FixtureScope::Session,
        ];

        // Insert a value into every scope
        for scope in &scopes {
            cache.insert("val".to_string(), scope, serde_json::json!(scope.to_string()));
        }

        // Clear each scope one by one and verify the others remain
        for (i, scope) in scopes.iter().enumerate() {
            cache.clear_scope(scope);
            assert_eq!(cache.scope_len(scope), 0);
            // All subsequent scopes should still have their values
            for remaining_scope in &scopes[i + 1..] {
                assert_eq!(cache.scope_len(remaining_scope), 1);
            }
        }
    }
}
