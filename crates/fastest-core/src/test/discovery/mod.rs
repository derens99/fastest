//! Test Discovery Module - Optimized for Maximum Performance
//!
//! High-performance test discovery using:
//! - Thread-local parsers to eliminate allocation overhead
//! - Memory-mapped files for large file reads
//! - SIMD-accelerated pattern matching
//! - Smart work distribution for optimal parallelism
//! - Minimal allocations with string interning

use crate::error::Result;
use crate::test::parametrize::expand_parametrized_tests;
use crate::test::parser::Parser as TsParser;
use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use ignore::WalkBuilder;
use memmap2::Mmap;
use once_cell::sync::Lazy;
// use parking_lot::RwLock; // Not needed here
use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use unicode_normalization::UnicodeNormalization;

/// String interner for reducing memory allocations
type InternedString = Arc<str>;

thread_local! {
    /// Thread-local string interner to reduce allocations
    static STRING_INTERNER: RefCell<HashMap<String, InternedString>> = RefCell::new(HashMap::new());
    
    /// Thread-local tree-sitter parser for zero allocation overhead
    static TREE_SITTER_PARSER: RefCell<Option<TsParser>> = const { RefCell::new(None) };
}

/// Intern a string to reduce memory usage
fn intern_string(s: &str) -> InternedString {
    STRING_INTERNER.with(|interner| {
        let mut map = interner.borrow_mut();
        if let Some(interned) = map.get(s) {
            Arc::clone(interned)
        } else {
            let interned: InternedString = Arc::from(s);
            map.insert(s.to_string(), Arc::clone(&interned));
            interned
        }
    })
}

/// Test item representing a discovered test - optimized for memory efficiency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestItem {
    pub id: String, // Keep as String for serialization compatibility
    pub path: PathBuf,
    pub function_name: String,
    pub line_number: Option<u32>, // Changed from usize to u32 to save memory
    pub decorators: SmallVec<[String; 2]>, // Most tests have 0-2 decorators
    pub is_async: bool,
    pub fixture_deps: SmallVec<[String; 4]>, // Most tests have <4 fixtures
    pub class_name: Option<String>,
    pub is_xfail: bool,
    pub name: String,
    /// Map of parameter names to whether they are indirect
    #[serde(default)]
    pub indirect_params: HashMap<String, bool>,
}

/// Test metadata packed into a smaller structure
#[derive(Debug, Clone, Copy)]
pub struct TestMetadata {
    pub line_number: u32,
    pub flags: u8, // Bit 0: is_async, Bit 1: is_xfail, Bit 2: has_class
}

impl TestMetadata {
    #[inline]
    pub fn is_async(&self) -> bool {
        self.flags & 0x01 != 0
    }
    
    #[inline]
    pub fn is_xfail(&self) -> bool {
        self.flags & 0x02 != 0
    }
    
    #[inline]
    pub fn has_class(&self) -> bool {
        self.flags & 0x04 != 0
    }
}

/// Pattern matcher for fast test detection
static TEST_PATTERNS: Lazy<AhoCorasick> = Lazy::new(|| {
    AhoCorasickBuilder::new()
        .match_kind(MatchKind::LeftmostFirst)
        .prefilter(true) // Enable prefilter for better performance
        .byte_classes(true) // Reduce automaton size
        .build(&[
            "def test_",
            "async def test_",
            "class Test",
        ])
        .unwrap()
});

/// Regex for pytest file patterns
static PYTEST_FILE_RE: Lazy<Regex> = 
    Lazy::new(|| Regex::new(r"(?i)^(test_.*|.*_test)\.py$").unwrap());

/// Fast test discovery using optimized strategies
pub fn discover_tests(paths: &[PathBuf]) -> Result<Vec<TestItem>> {
    discover_tests_with_filtering(paths, false)
}

/// Discover tests with optimal parallelization
pub fn discover_tests_with_filtering(
    paths: &[PathBuf],
    _apply_performance_filtering: bool, // Keep for API compatibility
) -> Result<Vec<TestItem>> {
    // Collect test files efficiently
    let test_files = collect_test_files(paths);
    
    if test_files.is_empty() {
        return Ok(Vec::new());
    }
    
    // Calculate optimal chunk size based on CPU cache and thread count
    let num_threads = rayon::current_num_threads();
    let min_chunk_size = 16; // Minimum to avoid excessive overhead
    let chunk_size = (test_files.len() / (num_threads * 4)).max(min_chunk_size);
    
    // Pre-allocate with estimated capacity
    let estimated_tests_per_file = 5;
    let estimated_capacity = test_files.len() * estimated_tests_per_file;
    
    // Process files in parallel with optimized chunking
    let tests: Vec<TestItem> = test_files
        .par_chunks(chunk_size)
        .map(|chunk| {
            let mut batch_results = Vec::with_capacity(chunk.len() * estimated_tests_per_file);
            
            for path in chunk {
                if let Ok(mut file_tests) = discover_tests_in_file(path) {
                    batch_results.append(&mut file_tests);
                }
            }
            batch_results
        })
        .flatten()
        .collect();
    
    Ok(tests)
}

/// Collect test files with efficient walking
fn collect_test_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths
        .par_iter()
        .flat_map(|path| {
            if path.is_file() {
                if is_python_test_file(path) {
                    vec![path.clone()]
                } else {
                    vec![]
                }
            } else {
                // Use ignore crate for fast directory walking
                WalkBuilder::new(path)
                    .standard_filters(false)
                    .hidden(false)
                    .git_ignore(true)
                    .git_exclude(true)
                    .follow_links(false)
                    .max_depth(None)
                    .filter_entry(|entry| {
                        // Skip __pycache__ directories
                        entry.file_name() != "__pycache__"
                    })
                    .build()
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| entry.file_type().map_or(false, |ft| ft.is_file()))
                    .filter(|entry| is_python_test_file(entry.path()))
                    .map(|entry| entry.path().to_path_buf())
                    .collect::<Vec<_>>()
            }
        })
        .collect()
}

/// Check if file is a Python test file
#[inline]
fn is_python_test_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map_or(false, |ext| ext == "py")
        && path.file_name()
            .and_then(|name| name.to_str())
            .map_or(false, |name| PYTEST_FILE_RE.is_match(name))
}

/// Discover tests in a single file with optimal strategy
fn discover_tests_in_file(file_path: &Path) -> Result<Vec<TestItem>> {
    // Try memory-mapped file for large files
    let file_size = std::fs::metadata(file_path)
        .map(|m| m.len())
        .unwrap_or(0);
    
    // Use mmap for files > 1MB
    if file_size > 1_048_576 {
        discover_tests_mmap(file_path)
    } else {
        // Read small files normally
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| crate::error::Error::Discovery(
                format!("Failed to read {}: {}", file_path.display(), e)
            ))?;
        discover_tests_in_content(file_path, &content)
    }
}

/// Discover tests using memory-mapped file
fn discover_tests_mmap(file_path: &Path) -> Result<Vec<TestItem>> {
    let file = File::open(file_path)
        .map_err(|e| crate::error::Error::Discovery(
            format!("Failed to open {}: {}", file_path.display(), e)
        ))?;
    
    let mmap = unsafe { Mmap::map(&file) }
        .map_err(|e| crate::error::Error::Discovery(
            format!("Failed to mmap {}: {}", file_path.display(), e)
        ))?;
    
    // Quick check if file contains test patterns
    if !TEST_PATTERNS.is_match(&mmap) {
        return Ok(Vec::new());
    }
    
    // Convert to string for parsing
    let content = std::str::from_utf8(&mmap)
        .map_err(|e| crate::error::Error::Discovery(
            format!("Invalid UTF-8 in {}: {}", file_path.display(), e)
        ))?;
    
    discover_tests_in_content(file_path, content)
}

/// Discover tests in file content using thread-local parser
fn discover_tests_in_content(file_path: &Path, content: &str) -> Result<Vec<TestItem>> {
    // Use thread-local parser to avoid allocation
    TREE_SITTER_PARSER.with(|parser_cell| {
        let mut parser_opt = parser_cell.borrow_mut();
        if parser_opt.is_none() {
            *parser_opt = Some(TsParser::new()?);
        }
        let parser = parser_opt.as_mut().unwrap();
        
        // Parse content
        let (_, tests, _, _) = parser.parse_content(content)?;
        
        // Pre-allocate result vector
        let mut items = Vec::with_capacity(tests.len() * 2);
        
        for test in tests {
            let base_id = create_test_id(file_path, &test.name, test.class_name.as_deref());
            
            // Convert decorators and fixtures to SmallVec
            let decorators = test.decorators.into_iter().collect();
            let fixture_deps = test.parameters.into_iter().collect();
            
            let base_test = TestItem {
                id: base_id,
                path: file_path.to_path_buf(),
                name: test.name.clone(),
                function_name: test.name,
                line_number: Some(test.line_number as u32),
                decorators,
                is_async: test.is_async,
                fixture_deps,
                class_name: test.class_name,
                is_xfail: false,
                indirect_params: HashMap::new(),
            };
            
            // Expand parametrized tests
            let expanded = expand_parametrized_tests(&base_test, &base_test.decorators)?;
            items.extend(expanded);
        }
        
        Ok(items)
    })
}

/// Create normalized test ID
#[inline]
fn create_test_id(file_path: &Path, function_name: &str, class_name: Option<&str>) -> String {
    let normalized_function = normalize_unicode(function_name);
    
    if let Some(class) = class_name {
        let normalized_class = normalize_unicode(class);
        format!("{}::{}::{}", file_path.display(), normalized_class, normalized_function)
    } else {
        format!("{}::{}", file_path.display(), normalized_function)
    }
}

/// Normalize unicode strings for consistent test IDs
#[inline]
fn normalize_unicode(s: &str) -> String {
    // First normalize to NFC (canonical composition)
    let normalized: String = s.nfc().collect();
    
    // Create safe ID by replacing non-ASCII characters
    normalized
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c.to_string()
            } else if c == ' ' || c == '-' || c == '.' || c == ':' {
                "_".to_string()
            } else {
                // Convert non-ASCII to hex representation
                format!("_u{:04x}", c as u32)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[test]
    fn test_discover_simple_tests() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test_example.py");
        
        fs::write(&test_file, r#"
def test_one():
    pass

async def test_two():
    pass

class TestClass:
    def test_three(self):
        pass
"#).unwrap();
        
        let tests = discover_tests(&[temp_dir.path().to_path_buf()]).unwrap();
        assert_eq!(tests.len(), 3);
    }
    
    #[test]
    fn test_string_interning() {
        let s1 = intern_string("test");
        let s2 = intern_string("test");
        assert!(Arc::ptr_eq(&s1, &s2)); // Same allocation
    }
}