# Critical Bug Fixes for Fastest

## 1. Fixture Teardown Timing Issue

### Problem
When transitioning between test classes, `teardown_class` may be called at incorrect times:
- Too early: Before all tests in the class have completed
- Too late or not at all: If exceptions occur during test execution
- Out of order: With respect to class-scoped fixtures

### Root Cause
The current implementation tracks class transitions but doesn't properly handle:
1. Exceptions during test execution that prevent proper teardown
2. Class-scoped fixtures that need teardown coordination
3. Nested class hierarchies

### Solution
Implement a robust class lifecycle manager that:
1. Tracks all active classes and their states
2. Ensures teardown happens in reverse setup order
3. Handles exceptions gracefully
4. Coordinates with fixture teardown

### Implementation Plan
```python
# Enhanced class lifecycle tracking
class ClassLifecycleManager:
    def __init__(self):
        self.active_classes = OrderedDict()  # class_path -> ClassState
        self.setup_order = []  # Track setup order for proper teardown
    
    def setup_class(self, class_path, cls):
        """Setup a class and track it"""
        if class_path in self.active_classes:
            return  # Already setup
        
        # Call setup_class
        if hasattr(cls, 'setup_class'):
            cls.setup_class()
        
        self.active_classes[class_path] = ClassState(cls, setup=True)
        self.setup_order.append(class_path)
    
    def teardown_all_classes(self):
        """Teardown all classes in reverse order"""
        for class_path in reversed(self.setup_order):
            if class_path in self.active_classes:
                state = self.active_classes[class_path]
                if state.setup and not state.teardown:
                    self._teardown_class(class_path, state.cls)
    
    def transition_to_class(self, new_class_path, new_cls):
        """Handle transition between classes"""
        # Don't teardown if staying in same class
        if self.current_class_path == new_class_path:
            return
        
        # Teardown previous class if needed
        if self.current_class_path:
            self._teardown_if_last_test()
        
        # Setup new class
        self.setup_class(new_class_path, new_cls)
```

## 2. Unicode Handling in Test Names

### Problem
Test names containing Unicode characters (emojis, non-ASCII) fail during:
- Test discovery and parsing
- Test ID generation
- Result reporting

### Root Cause
1. Improper UTF-8 encoding/decoding in Rust-Python boundary
2. Test ID generation doesn't normalize Unicode
3. JSON serialization issues with Unicode

### Solution
1. Ensure UTF-8 handling throughout the codebase
2. Normalize Unicode in test IDs using NFD normalization
3. Properly escape Unicode in JSON serialization

### Implementation Plan
```rust
// In fastest-core/src/test/parser/tree_sitter.rs
pub fn normalize_test_name(name: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    
    // Normalize to NFD (canonical decomposition)
    let normalized = name.nfd().collect::<String>();
    
    // Replace problematic characters for test IDs
    normalized
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                // Convert non-ASCII to hex representation
                format!("_u{:04x}", c as u32).chars().collect::<String>()
            }
        })
        .collect()
}

// In Python worker
def safe_test_id(test_name):
    """Create a safe test ID from Unicode test name"""
    import unicodedata
    
    # Normalize Unicode
    normalized = unicodedata.normalize('NFD', test_name)
    
    # Keep original for display, use safe version for ID
    safe_id = ''.join(
        c if c.isascii() and (c.isalnum() or c == '_') 
        else f'_u{ord(c):04x}'
        for c in normalized
    )
    
    return safe_id, test_name  # Return both safe ID and display name
```

## 3. Memory Management in Cache System

### Problem
The discovery cache system has memory leaks:
- Old cache entries are not properly cleaned up
- Large test suites cause unbounded memory growth
- Arc<Vec<TestItem>> references prevent garbage collection

### Root Cause
1. No cache eviction policy
2. Shared references (Arc) prevent deallocation
3. No memory limits on cache size

### Solution
Implement a bounded LRU cache with memory limits:

### Implementation Plan
```rust
// In fastest-core/src/cache.rs
use lru::LruCache;
use std::num::NonZeroUsize;

pub struct BoundedDiscoveryCache {
    entries: LruCache<PathBuf, CacheEntry>,
    memory_limit: usize,
    current_memory: AtomicUsize,
}

impl BoundedDiscoveryCache {
    pub fn new(max_entries: usize, memory_limit: usize) -> Self {
        Self {
            entries: LruCache::new(NonZeroUsize::new(max_entries).unwrap()),
            memory_limit,
            current_memory: AtomicUsize::new(0),
        }
    }
    
    pub fn insert(&mut self, path: PathBuf, entry: CacheEntry) {
        let entry_size = Self::estimate_size(&entry);
        
        // Evict entries if over memory limit
        while self.current_memory.load(Ordering::Relaxed) + entry_size > self.memory_limit {
            if let Some((_, evicted)) = self.entries.pop_lru() {
                let evicted_size = Self::estimate_size(&evicted);
                self.current_memory.fetch_sub(evicted_size, Ordering::Relaxed);
            } else {
                break;
            }
        }
        
        self.entries.put(path, entry);
        self.current_memory.fetch_add(entry_size, Ordering::Relaxed);
    }
    
    fn estimate_size(entry: &CacheEntry) -> usize {
        // Estimate memory usage of cache entry
        std::mem::size_of::<CacheEntry>() 
            + entry.tests.capacity() * std::mem::size_of::<TestItem>()
            + entry.tests.iter()
                .map(|t| t.name.len() + t.path.as_os_str().len())
                .sum::<usize>()
    }
}
```

## 4. Error Propagation from Python Subprocess

### Problem
Errors from Python subprocesses are lost or poorly formatted:
- Stack traces are truncated
- Unicode errors in error messages
- Lost context about which test failed

### Root Cause
1. Subprocess communication uses basic stdout/stderr capture
2. Error serialization doesn't preserve full context
3. No structured error protocol between Rust and Python

### Solution
Implement structured error communication protocol:

### Implementation Plan
```python
# In Python worker
class StructuredError:
    def __init__(self, exc_type, exc_value, exc_tb):
        import traceback
        
        self.type = exc_type.__name__
        self.message = str(exc_value)
        self.traceback = traceback.format_tb(exc_tb)
        self.locals = self._extract_locals(exc_tb)
    
    def _extract_locals(self, tb):
        """Extract local variables from traceback"""
        locals_dict = {}
        while tb:
            frame = tb.tb_frame
            # Only include test function locals
            if 'test_' in frame.f_code.co_name:
                locals_dict[frame.f_code.co_name] = {
                    k: repr(v)[:100]  # Limit size
                    for k, v in frame.f_locals.items()
                    if not k.startswith('_')
                }
            tb = tb.tb_next
        return locals_dict
    
    def to_json(self):
        return {
            'error_type': 'structured',
            'exc_type': self.type,
            'exc_message': self.message,
            'traceback': self.traceback,
            'locals': self.locals,
        }
```

## 5. Plugin Loading Order Issues

### Problem
Plugins load in inconsistent order causing:
- Hook execution order problems
- Fixture override issues
- Configuration conflicts

### Root Cause
1. HashMap iteration is non-deterministic
2. No explicit priority system
3. No dependency resolution between plugins

### Solution
Implement deterministic plugin loading with priorities:

### Implementation Plan
```rust
// In fastest-plugins/src/loader.rs
#[derive(Debug, Clone)]
pub struct PluginLoadOrder {
    /// Builtin plugins (highest priority)
    builtin: Vec<String>,
    /// User plugins from conftest.py
    conftest: Vec<String>,
    /// Installed plugins
    installed: Vec<String>,
    /// CLI specified plugins (lowest priority)
    cli: Vec<String>,
}

impl PluginLoadOrder {
    pub fn iter_in_order(&self) -> impl Iterator<Item = &String> {
        self.builtin.iter()
            .chain(self.conftest.iter())
            .chain(self.installed.iter())
            .chain(self.cli.iter())
    }
    
    pub fn get_priority(&self, plugin_name: &str) -> usize {
        if self.builtin.contains(&plugin_name.to_string()) {
            return 0;  // Highest priority
        }
        if self.conftest.contains(&plugin_name.to_string()) {
            return 1;
        }
        if self.installed.contains(&plugin_name.to_string()) {
            return 2;
        }
        3  // Lowest priority
    }
}
```

## Testing Strategy

1. **Unit Tests**: Add tests for each bug fix
2. **Integration Tests**: Test complete scenarios
3. **Regression Tests**: Ensure fixes don't break existing functionality
4. **Performance Tests**: Verify no performance degradation

## Rollout Plan

1. **Phase 1**: Fix critical bugs (teardown timing, unicode)
2. **Phase 2**: Fix memory and error handling
3. **Phase 3**: Fix plugin loading order
4. **Phase 4**: Comprehensive testing
5. **Phase 5**: Release as v1.0.11