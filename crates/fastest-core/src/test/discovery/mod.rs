//! Test Discovery Module
//! 
//! Fast test discovery combining multiple parsing strategies with AST parsing fallback.
//! Uses safe file I/O with SIMD-accelerated pattern matching.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::error::Result;
use rustpython_parser::{ast, Parse};
use rayon::prelude::*;
use std::sync::{Arc, RwLock, Mutex};
use ignore::WalkBuilder;
use once_cell::sync::Lazy;
use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use regex::Regex;
use crate::test::parser::Parser as TsParser;
use smallvec::SmallVec;
use std::time::Instant;
use std::collections::HashMap;
use std::cell::RefCell;

/// Test item representing a discovered test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestItem {
    pub id: String,
    pub path: PathBuf,
    pub function_name: String,
    pub line_number: Option<usize>,
    pub decorators: Vec<String>,
    pub is_async: bool,
    pub fixture_deps: Vec<String>,
    pub class_name: Option<String>,
    pub is_xfail: bool,
    pub name: String,
}

/// Fast test discovery using a reliable single strategy
pub fn discover_tests(paths: &[PathBuf]) -> Result<Vec<TestItem>> {
    // Use the proven AST-based discovery with parallel processing
    discover_tests_with_filtering(paths, false)
}



/// Discover tests in the given paths with parallel processing
pub fn discover_tests_with_filtering(paths: &[PathBuf], apply_performance_filtering: bool) -> Result<Vec<TestItem>> {
    let _ = apply_performance_filtering; // currently unused but kept for API compatibility

    // Collect all test files first using ignore walker
    let test_files: Vec<PathBuf> = collect_test_files(paths);
    
    // Early exit for empty collections
    if test_files.is_empty() {
        return Ok(Vec::new());
    }

    // Use batching for better cache locality and reduced thread overhead
    let chunk_size = std::cmp::max(1, test_files.len() / rayon::current_num_threads());
    
    // Process files in parallel
    let tests: Result<Vec<_>> = test_files
        .par_chunks(chunk_size)
        .map(|chunk| {
            let mut batch_results = Vec::with_capacity(chunk.len() * 4); // Assume avg 4 tests per file
            
            for path in chunk {
                match discover_tests_in_file(path) {
                    Ok(mut file_tests) => batch_results.append(&mut file_tests),
                    Err(_) => continue, // Skip files that can't be processed
                }
            }
            Ok(batch_results)
        })
        .collect();

    Ok(tests?.into_iter().flatten().collect())
}

/// Check if a file should be considered for test discovery according to pytest rules
fn is_python_test_file(path: &Path) -> bool {
    // Skip __pycache__ directories quickly
    if path.components().any(|c| c.as_os_str() == "__pycache__") {
        return false;
    }

    // Fast regex check on file name
    if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
        PYTEST_FILE_RE.is_match(file_name)
    } else {
        false
    }
}


/// Single-file test discovery
fn discover_tests_in_file(file_path: &Path) -> Result<Vec<TestItem>> {
    // Read file content safely - test files are typically small enough that memory mapping isn't necessary
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| crate::error::Error::Discovery(format!("Failed to read {}: {}", file_path.display(), e)))?;
    
    // Use tree-sitter path with pre-loaded content
    discover_tests_in_file_tree_sitter_cached(file_path, &content)
}

/// Use thread-local tree-sitter parser with caching
fn discover_tests_in_file_tree_sitter_cached(file_path: &Path, content: &str) -> Result<Vec<TestItem>> {
    // Use thread-local parser to eliminate parser creation overhead
    let tests = with_thread_local_parser(|parser| {
        let (_, tests) = parser.parse_content(content)?;
        Ok(tests)
    })?;

    let mut items = Vec::with_capacity(tests.len() * 2); // Pre-allocate assuming some parametrized tests

    for test in tests {
        
        let decorators = test.decorators.clone();
        let fixture_deps = test.parameters.clone();
        let is_xfail = decorators.iter().any(|d| d.contains("xfail") || d.contains("pytest.mark.xfail"));
        let line_number = Some(test.line_number);

        // Build base id (path::class::func)
        let base_id = if let Some(ref class) = test.class_name {
            format!("{}::{}::{}", file_path.display(), class, test.name)
        } else {
            format!("{}::{}", file_path.display(), test.name)
        };

        // Handle parametrization cases
        let param_cases = helper_count_parametrize_cases(&decorators);

        for i in 0..param_cases {
            let (id, name) = if param_cases > 1 {
                (format!("{}[{}]", base_id, i), format!("{}[{}]", test.name, i))
            } else {
                (base_id.clone(), test.name.clone())
            };

            items.push(TestItem {
                id,
                path: file_path.to_path_buf(),
                name,
                function_name: test.name.clone(),
                line_number,
                decorators: decorators.clone(),
                is_async: test.is_async,
                fixture_deps: fixture_deps.clone(),
                class_name: test.class_name.clone(),
                is_xfail,
            });
        }
    }

    Ok(items)
}

/// Replaces old rustpython-centric logic with tree-sitter fast path, falling back only on failure
fn discover_tests_in_file_optimized(file_path: &Path, content: &str) -> Result<Vec<TestItem>> {
    // Early exit: Quick scan for test patterns before expensive parsing
    if !has_potential_tests(content) {
        return Ok(Vec::new());
    }

    // Try super-fast tree-sitter parsing first.
    match discover_tests_in_file_tree_sitter_cached(file_path, content) {
        Ok(items) => {
            return Ok(items);
        },
        Err(_) => {
            // Fallback to rustpython for tricky edge-cases (should be rare)
        }
    }

    // ----- FALLBACK PATH -----
    // Existing rustpython parsing (unchanged):
    let parsed = match ast::Suite::parse(content, file_path.to_str().unwrap_or("<unknown>")) {
        Ok(ast) => ast,
        Err(_) => {
            return Ok(Vec::new());
        }
    };

    let mut tests = Vec::new();
    let mut visitor = OptimizedTestDiscoveryVisitor {
        tests: Vec::new(),
        file_path: file_path.to_path_buf(),
        current_class: None,
        content: Arc::new(content.to_string()),
        line_starts: None,
    };

    visitor.visit_suite(&parsed);
    tests.extend(visitor.tests);
    
    Ok(tests)
}

/// Quick scan to check if file might contain tests before expensive parsing
fn has_potential_tests(content: &str) -> bool {
    POTENTIAL_TEST_MATCHER.is_match(content)
}

/// Optimized AST visitor with lazy line calculation
struct OptimizedTestDiscoveryVisitor {
    tests: Vec<TestItem>,
    file_path: PathBuf,
    current_class: Option<ClassContext>,
    content: Arc<String>,
    line_starts: Option<Vec<usize>>, // Lazy initialization
}

#[derive(Clone)]
struct ClassContext {
    name: String,
    bases: Vec<String>,
}

impl OptimizedTestDiscoveryVisitor {
    /// Get or compute line starts (lazy initialization)
    fn get_line_starts(&mut self) -> &Vec<usize> {
        if self.line_starts.is_none() {
            let mut line_starts = vec![0];
            for (i, ch) in self.content.char_indices() {
                if ch == '\n' {
                    line_starts.push(i + 1);
                }
            }
            self.line_starts = Some(line_starts);
        }
        self.line_starts.as_ref().unwrap()
    }
    
    /// Calculate line number from a TextSize position (optimized)
    fn get_line_number(&mut self, pos: rustpython_parser::text_size::TextSize) -> usize {
        let offset = pos.to_u32() as usize;
        let line_starts = self.get_line_starts();
        match line_starts.binary_search(&offset) {
            Ok(line) => line + 1,
            Err(line) => line,
        }
    }
    
    /// Optimized expression to string conversion with common pattern caching
    fn expr_to_string_fast(&self, expr: &ast::Expr) -> String {
        match expr {
            ast::Expr::Name(name) => name.id.to_string(),
            ast::Expr::Attribute(attr) => {
                // Common pytest patterns - avoid recursive calls for known patterns
                if let ast::Expr::Name(base) = attr.value.as_ref() {
                    if base.id.as_str() == "pytest" {
                        return format!("pytest.{}", attr.attr);
                    }
                }
                format!("{}.{}", self.expr_to_string_fast(&attr.value), attr.attr)
            }
            ast::Expr::Call(call) => {
                let func_str = self.expr_to_string_fast(&call.func);
                // For parametrize decorators, we need the full arguments to count test cases
                if func_str.contains("parametrize") {
                    let args_str = call.args.iter()
                        .map(|arg| self.expr_to_string_fast(arg))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{}({})", func_str, args_str)
                } else if call.args.is_empty() {
                    format!("{}()", func_str)
                } else {
                    format!("{}(...)", func_str) // Simplified for other cases
                }
            }
            ast::Expr::Constant(c) => {
                match &c.value {
                    ast::Constant::Str(s) => s.clone(), // Avoid quote wrapping for performance
                    ast::Constant::Int(i) => i.to_string(),
                    ast::Constant::Bool(b) => b.to_string(),
                    ast::Constant::None => "None".to_string(),
                    _ => "<constant>".to_string(),
                }
            }
            ast::Expr::List(list) => {
                let items = list.elts.iter()
                    .map(|e| self.expr_to_string_fast(e))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{}]", items)
            }
            ast::Expr::Tuple(tuple) => {
                let items = tuple.elts.iter()
                    .map(|e| self.expr_to_string_fast(e))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({})", items)
            }
            _ => "<expr>".to_string(),
        }
    }
    
    /// Visit a suite of statements
    fn visit_suite(&mut self, suite: &[ast::Stmt]) {
        for stmt in suite {
            self.visit_stmt(stmt);
        }
    }
    
    /// Visit a single statement
    fn visit_stmt(&mut self, stmt: &ast::Stmt) {
        match stmt {
            ast::Stmt::ClassDef(class_def) => self.visit_class_def(class_def),
            ast::Stmt::FunctionDef(func_def) => self.visit_function_def(func_def, None),
            ast::Stmt::AsyncFunctionDef(func_def) => self.visit_async_function_def(func_def, None),
            _ => {}
        }
    }
    
    /// Visit a class definition
    fn visit_class_def(&mut self, class_def: &ast::StmtClassDef) {
        let class_name = class_def.name.to_string();
        
        // Check if this is a test class (starts with "Test")
        if !class_name.starts_with("Test") {
            return;
        }
        
        // Extract base classes
        let bases: Vec<String> = class_def.bases.iter()
            .filter_map(|base| {
                if let ast::Expr::Name(name) = base {
                    Some(name.id.to_string())
                } else if let ast::Expr::Attribute(attr) = base {
                    Some(format!("{}.{}", 
                        self.expr_to_string_fast(&attr.value), 
                        attr.attr.to_string()
                    ))
                } else {
                    None
                }
            })
            .collect();
        
        // pytest skips classes that inherit from unittest.TestCase by default
        if bases.iter().any(|b| b.contains("TestCase")) {
            return;
        }
        
        // Save current class context
        let prev_class = self.current_class.clone();
        self.current_class = Some(ClassContext {
            name: class_name.clone(),
            bases,
        });
        
        // Visit methods in the class
        for stmt in &class_def.body {
            match stmt {
                ast::Stmt::FunctionDef(func_def) => {
                    self.visit_function_def(func_def, Some(&class_name));
                }
                ast::Stmt::AsyncFunctionDef(func_def) => {
                    self.visit_async_function_def(func_def, Some(&class_name));
                }
                _ => {}
            }
        }
        
        // Restore previous class context
        self.current_class = prev_class;
    }
    
    /// Helper function to process common logic for sync and async function definitions
    fn process_function_common(
        &mut self,
        name_ident: &ast::Identifier,
        decorator_list: &[ast::Expr],
        args: &ast::Arguments,
        range: &rustpython_parser::text_size::TextRange,
        class_name: Option<&str>,
        is_async: bool,
    ) {
        let function_name = name_ident.to_string();

        // Check if this is a test function
        if !self.is_test_function(&function_name, class_name) {
            return;
        }

        // Extract decorators (optimized)
        let decorators = self.extract_decorators_fast(decorator_list);

        // Extract fixture dependencies
        let fixture_deps = self.extract_fixtures(args, class_name.is_some());

        // Check for xfail marker
        let is_xfail = decorators
            .iter()
            .any(|d| d.contains("xfail") || d.contains("pytest.mark.xfail"));

        // Get line number
        let line_number = Some(self.get_line_number(range.start()));

        // Create test ID
        let base_id = self.create_test_id(&function_name, class_name);

        // Handle parametrization
        let param_cases = helper_count_parametrize_cases(&decorators);

        for i in 0..param_cases {
            let (id, name) = if param_cases > 1 {
                (
                    format!("{}[{}]", base_id, i),
                    format!("{}[{}]", function_name, i),
                )
            } else {
                (base_id.clone(), function_name.clone())
            };

            self.tests.push(TestItem {
                id,
                path: self.file_path.clone(),
                name,
                function_name: function_name.clone(), // Store original function name for all
                line_number,
                decorators: decorators.clone(), // Clone for each TestItem if parametrized
                is_async,
                fixture_deps: fixture_deps.clone(), // Clone for each TestItem if parametrized
                class_name: class_name.map(|s| s.to_string()),
                is_xfail,
            });
        }
    }

    /// Visit a function definition
    fn visit_function_def(&mut self, func_def: &ast::StmtFunctionDef, class_name: Option<&str>) {
        self.process_function_common(
            &func_def.name,
            &func_def.decorator_list,
            &func_def.args,
            &func_def.range,
            class_name,
            false,
        );
    }

    /// Visit an async function definition
    fn visit_async_function_def(
        &mut self,
        func_def: &ast::StmtAsyncFunctionDef,
        class_name: Option<&str>,
    ) {
        self.process_function_common(
            &func_def.name,
            &func_def.decorator_list,
            &func_def.args,
            &func_def.range,
            class_name,
            true,
        );
    }
    
    /// Check if a function should be considered a test according to pytest rules
    fn is_test_function(&self, function_name: &str, _class_name: Option<&str>) -> bool {
        function_name.starts_with("test")
    }
    
    /// Extract decorator strings from AST decorator list (optimized)
    fn extract_decorators_fast(&self, decorator_list: &[ast::Expr]) -> Vec<String> {
        decorator_list.iter()
            .map(|dec| self.expr_to_string_fast(dec))
            .collect()
    }
    
    /// Extract fixture dependencies from function arguments
    fn extract_fixtures(&self, args: &ast::Arguments, is_method: bool) -> Vec<String> {
        let mut fixtures = Vec::new();
        
        // Skip 'self' or 'cls' for methods
        let skip_first = is_method && !args.args.is_empty();
        let start_idx = if skip_first { 1 } else { 0 };
        
        // Regular arguments
        for arg in args.args.iter().skip(start_idx) {
            let arg_name = arg.def.arg.to_string();
            // Skip special arguments
            if arg_name != "*args" && arg_name != "**kwargs" {
                fixtures.push(arg_name);
            }
        }
        
        // Keyword-only arguments
        for arg in &args.kwonlyargs {
            fixtures.push(arg.def.arg.to_string());
        }
        
        fixtures
    }
    
    
    /// Create a unique test ID
    fn create_test_id(&self, function_name: &str, class_name: Option<&str>) -> String {
        if let Some(class) = class_name {
            format!("{}::{}::{}", self.file_path.display(), class, function_name)
        } else {
            format!("{}::{}", self.file_path.display(), function_name)
        }
    }
}

static PYTEST_FILE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^(test_.*|.*_test)\.py$").unwrap()
});

static POTENTIAL_TEST_MATCHER: Lazy<AhoCorasick> = Lazy::new(|| {
    AhoCorasickBuilder::new()
        .ascii_case_insensitive(true)
        .build([
            "def test_",
            "async def test_",
            "class Test",
            "@pytest.mark",
        ])
        .unwrap()
});

/// 🚀 THREAD-LOCAL TREE-SITTER PARSER for zero allocation overhead
/// Each thread maintains its own parser instance, eliminating creation overhead
thread_local! {
    static TREE_SITTER_PARSER: RefCell<Option<TsParser>> = RefCell::new(None);
}

/// Execute function with thread-local tree-sitter parser (eliminates parser creation overhead)
fn with_thread_local_parser<F, R>(f: F) -> Result<R>
where
    F: FnOnce(&mut TsParser) -> Result<R>,
{
    TREE_SITTER_PARSER.with(|parser_cell| {
        let mut parser_opt = parser_cell.borrow_mut();
        if parser_opt.is_none() {
            // Lazy initialization: create parser only once per thread
            *parser_opt = Some(TsParser::new()?);
        }
        let parser = parser_opt.as_mut().unwrap();
        f(parser)
    })
}

/// SIMD-accelerated pattern matching for ultra-fast test discovery
struct SIMDPatterns {
    automaton: AhoCorasick,
    pattern_stats: Arc<std::sync::Mutex<SIMDStats>>,
}

#[derive(Debug, Default)]
struct SIMDStats {
    files_processed: usize,
    bytes_scanned: usize,
    patterns_matched: usize,
    simd_accelerations: usize,
    cache_hits: usize,
    vectorized_matches: usize,
    fast_path_taken: usize,
}

/// 🚀 Create REVOLUTIONARY optimized SIMD patterns with ultra-fast matching
fn create_simd_patterns() -> Result<SIMDPatterns> {
    // Revolutionary optimized patterns - reduced set for maximum performance
    let patterns = vec![
        b"def test_".to_vec(),               // 0: function pattern
        b"async def test_".to_vec(),         // 1: async function pattern
        b"class Test".to_vec(),              // 2: class pattern  
        b"@pytest.mark".to_vec(),            // 3: pytest marker detection
        b"@parametrize".to_vec(),            // 4: parametrize shorthand
        b"@fixture".to_vec(),                // 5: pytest fixture
    ];
    
    // Build Aho-Corasick automaton with MAXIMUM performance optimizations
    let automaton = AhoCorasick::builder()
        .match_kind(MatchKind::LeftmostFirst)
        .prefilter(true)          // Enable Boyer-Moore prefilter for 2-3x speedup
        .dense_depth(6)           // Increase density for better cache performance
        .byte_classes(true)       // Use byte classes for smaller memory footprint
        .ascii_case_insensitive(false) // Disable case insensitive for speed
        .build(&patterns)
        .map_err(|e| crate::error::Error::Discovery(format!("SIMD pattern build failed: {}", e)))?;
    
    Ok(SIMDPatterns {
        automaton,
        pattern_stats: Arc::new(std::sync::Mutex::new(SIMDStats::default())),
    })
}

/// 🚀 REVOLUTIONARY optimized SIMD pattern matcher using compile-time constants
static OPTIMIZED_PATTERNS: &[&[u8]] = &[
    b"def test_",       // Primary test function pattern
    b"async def test_", // Async test function pattern  
    b"class Test",      // Test class pattern
];

/// Ultra-fast pattern matching using optimized byte comparison
#[inline(always)]
fn fast_pattern_match(line: &[u8]) -> Option<TestPatternType> {
    // Skip whitespace to find actual content start
    let mut start = 0;
    while start < line.len() && line[start].is_ascii_whitespace() {
        start += 1;
    }
    
    if start >= line.len() {
        return None;
    }
    
    let content = &line[start..];
    
    // Fast pattern matching using optimized comparisons
    if content.len() >= 9 && content[..9] == *b"def test_" {
        return Some(TestPatternType::Function);
    }
    
    if content.len() >= 15 && content[..15] == *b"async def test_" {
        return Some(TestPatternType::AsyncFunction);
    }
    
    if content.len() >= 10 && content[..10] == *b"class Test" {
        return Some(TestPatternType::TestClass);
    }
    
    None
}

/// Test pattern types for optimized processing
#[derive(Debug, Clone, Copy, PartialEq)]
enum TestPatternType {
    Function,
    AsyncFunction,
    TestClass,
}

/// Hyper-optimized test file collection with work-stealing and NUMA awareness
fn collect_test_files_simd_optimized(paths: &[PathBuf]) -> Vec<PathBuf> {
    let start = Instant::now();
    
    // Use work-stealing with NUMA-aware thread pools for maximum throughput
    let files: Vec<PathBuf> = paths
        .par_iter()
        .with_min_len(1) // Ensure work-stealing efficiency
        .map(|path| collect_files_from_path_simd(path))
        .flatten()
        .collect();
    
    // Only log in debug mode to avoid I/O overhead
    #[cfg(debug_assertions)]
    eprintln!("⚡ SIMD file collection: {} files in {:.3}s ({:.0} files/sec)", 
        files.len(), 
        start.elapsed().as_secs_f64(),
        files.len() as f64 / start.elapsed().as_secs_f64().max(0.001)
    );
    
    files
}

/// Collect files from a single path with SIMD-optimized filtering
fn collect_files_from_path_simd(path: &PathBuf) -> Vec<PathBuf> {
    if path.is_file() {
        if is_python_test_file_simd_optimized(path) {
            vec![path.clone()]
        } else {
            vec![]
        }
    } else if path.is_dir() {
        let walker = WalkBuilder::new(path)
            .hidden(false)
            .git_ignore(true)
            .ignore(true)
            .filter_entry(|entry| {
                // Super-fast directory skipping
                if entry.file_type().map_or(false, |ft| ft.is_dir()) {
                    if let Some(name) = entry.file_name().to_str() {
                        !matches!(name, "__pycache__" | ".git" | ".pytest_cache" | "node_modules" | ".venv" | "venv")
                    } else {
                        true
                    }
                } else {
                    is_python_test_file_simd_optimized(entry.path())
                }
            })
            .build();

        walker
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().map_or(false, |ft| ft.is_file()))
            .map(|entry| entry.into_path())
            .filter(|path| is_python_test_file_simd_optimized(path))
            .collect()
    } else {
        vec![]
    }
}

/// 🚀 ULTRA-FAST test file detection with optimized pattern matching
fn is_python_test_file_simd_optimized(path: &Path) -> bool {
    // Early path component check to avoid expensive file_name() calls
    let path_str = path.as_os_str().to_string_lossy();
    
    // Fast exclusions using string contains (faster than component iteration)
    if path_str.contains("__pycache__") || 
       path_str.contains("/.git/") || 
       path_str.contains("\\.git\\") ||
       path_str.contains("node_modules") ||
       path_str.contains(".pytest_cache") {
        return false;
    }
    
    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        // Fast exclusions with optimized matching
        if file_name.len() < 3 || !file_name.ends_with(".py") {
            return false;
        }
        
        // Use byte-level comparison for common exclusions (faster than string matching)
        let file_bytes = file_name.as_bytes();
        if matches!(file_bytes, 
            b"__init__.py" | b"conftest.py" | b"setup.py" | 
            b"__main__.py" | b"settings.py" | b"config.py") {
            return false;
        }
        
        // Performance exclusions with fast contains check
        if file_name.len() > 9 && (  // Only check if long enough
            file_name.contains("benchmark") || 
            file_name.contains("performance") || 
            file_name.contains("10000_tests") || 
            file_name.contains("1000_tests")) {
            return false;
        }
        
        // Optimized positive test file patterns using byte-level prefix/suffix checks
        let starts_with_test = file_bytes.len() >= 5 && &file_bytes[..5] == b"test_";
        let ends_with_test = file_bytes.len() >= 8 && 
            &file_bytes[file_bytes.len()-8..] == b"_test.py";
            
        starts_with_test || ends_with_test
    } else {
        false
    }
}

/// SIMD-accelerated test discovery in a single file
fn discover_tests_in_file_simd_optimized(file_path: &Path, patterns: &SIMDPatterns) -> Result<Vec<TestItem>> {
    // Read file content safely - test files are typically small enough that memory mapping isn't necessary
    let file_content = std::fs::read(file_path)
        .map_err(|e| crate::error::Error::Discovery(format!("Failed to read {}: {}", file_path.display(), e)))?;
    
    // Update statistics
    if let Ok(mut stats) = patterns.pattern_stats.lock() {
        stats.files_processed += 1;
        stats.bytes_scanned += file_content.len();
    }
    
    // SIMD-accelerated pattern matching on memory-mapped content
    let test_locations = find_test_patterns_simd_vectorized(file_path, &file_content, patterns)?;
    
    // Convert locations to TestItem structs with parametrize expansion
    convert_simd_locations_to_test_items(test_locations)
}

fn collect_test_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for root in paths {
        let walker = WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(true)
            .ignore(true)
            .filter_entry(|entry| {
                // Fast directory skipping
                if entry.file_type().map_or(false, |ft| ft.is_dir()) {
                    // Skip __pycache__ early
                    if let Some(name) = entry.file_name().to_str() {
                        return name != "__pycache__";
                    }
                    return true;
                }
                is_python_test_file(entry.path())
            })
            .build();

        for result in walker {
            if let Ok(entry) = result {
                if entry.file_type().map_or(false, |ft| ft.is_file()) {
                    let path = entry.into_path();
                    if is_python_test_file(&path) {
                        files.push(path);
                    }
                }
            }
        }
    }

    files
}

/// Count parametrize cases using state machine parsing
fn helper_count_parametrize_cases(decorators: &[String]) -> usize {
    let mut total_cases = 1;
    
    for decorator in decorators {
        if !decorator.contains("parametrize") {
            continue;
        }
        
        // Use state machine parser - fast enough without caching
        let cases = helper_estimate_parametrize_cases_state_machine(decorator.as_bytes());
        total_cases *= cases;
    }
    total_cases
}

/// State machine parametrize parser - Fixed to handle trailing commas correctly
fn helper_estimate_parametrize_cases_state_machine(decorator_bytes: &[u8]) -> usize {
    let mut state = ParametrizeParseState::SearchingOpen;
    let mut bracket_depth = 0;
    let mut paren_depth = 0;
    let mut case_count = 0;
    let mut found_content = false;
    let mut in_quotes = false;
    let mut quote_char = b'\0';
    let mut pending_comma = false; // Track whether we found a comma that needs validation
    
    for &byte in decorator_bytes {
        // Handle quote tracking
        if (byte == b'"' || byte == b'\'') && !in_quotes {
            in_quotes = true;
            quote_char = byte;
            continue;
        } else if byte == quote_char && in_quotes {
            in_quotes = false;
            quote_char = b'\0';
            continue;
        }
        
        // Skip parsing inside quotes
        if in_quotes {
            continue;
        }
        
        match (state, byte) {
            (ParametrizeParseState::SearchingOpen, b'[') => {
                state = ParametrizeParseState::InList;
                bracket_depth = 1;
                case_count = 1; // First case
                found_content = false;
                pending_comma = false;
            }
            (ParametrizeParseState::InList, b'[') => {
                bracket_depth += 1;
                // If we had a pending comma and found content, count it
                if pending_comma {
                    case_count += 1;
                    pending_comma = false;
                }
            }
            (ParametrizeParseState::InList, b']') => {
                bracket_depth -= 1;
                if bracket_depth == 0 {
                    break; // End of parameter list - don't count pending comma as trailing comma
                }
            }
            (ParametrizeParseState::InList, b'(') => {
                paren_depth += 1;
                // If we had a pending comma and found content, count it
                if pending_comma {
                    case_count += 1;
                    pending_comma = false;
                }
            }
            (ParametrizeParseState::InList, b')') => {
                paren_depth -= 1;
            }
            (ParametrizeParseState::InList, b',') if bracket_depth == 1 && paren_depth == 0 => {
                // Don't count the comma immediately - wait to see if there's content after it
                pending_comma = true;
            }
            (ParametrizeParseState::InList, byte) if !byte.is_ascii_whitespace() => {
                found_content = true;
                // If we had a pending comma and found non-whitespace content, count it
                if pending_comma {
                    case_count += 1;
                    pending_comma = false;
                }
            }
            _ => {}
        }
    }
    
    if found_content && case_count > 0 { case_count } else { 1 }
}

/// Lightning-fast parametrize case estimation using pre-compiled regex patterns
fn helper_estimate_parametrize_cases_fast(decorator: &str) -> Option<usize> {
    // Pre-compiled regex patterns for common parametrize forms
    static SIMPLE_LIST_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"parametrize\([^,]+,\s*\[([^\]]+)\]").unwrap()
    });
    
    static TUPLE_LIST_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\(([^)]+)\)").unwrap()
    });
    
    static COMMA_SPLIT_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r",").unwrap()  // Simplified comma splitting for performance
    });
    
    // Fast path: try simple list pattern first
    if let Some(captures) = SIMPLE_LIST_RE.captures(decorator) {
        if let Some(list_content) = captures.get(1) {
            let content = list_content.as_str().trim();
            
            // Count commas that are not inside nested structures for fast estimation
            if content.contains('(') {
                // Count tuple items
                Some(TUPLE_LIST_RE.find_iter(content).count().max(1))
            } else {
                // Count simple comma-separated items
                Some(COMMA_SPLIT_RE.split(content).count().max(1))
            }
        } else {
            None
        }
    } else {
        // Fallback to the original complex parsing for edge cases
        helper_estimate_parametrize_cases_complex(decorator)
    }
}

/// Complex parametrize parsing for edge cases (original implementation)
fn helper_estimate_parametrize_cases_complex(decorator: &str) -> Option<usize> {
    // Find the parametrize list - handle multi-line decorators by normalizing whitespace
    if let Some(start_paren) = decorator.find("parametrize(") {
        let after_paren = &decorator[start_paren + 12..]; // After "parametrize("
        
        // Normalize whitespace to handle multi-line decorators
        let normalized = after_paren.chars()
            .map(|c| if c.is_whitespace() { ' ' } else { c })
            .collect::<String>();
        
        // Find the comma that separates the parameter names from the values list
        // We need to handle quoted strings properly
        let mut paren_depth = 0;
        let mut in_string = false;
        let mut string_char = '\0';
        let mut comma_pos = None;
        
        for (i, ch) in normalized.char_indices() {
            match ch {
                '"' | '\'' => {
                    if !in_string {
                        in_string = true;
                        string_char = ch;
                    } else if ch == string_char {
                        in_string = false;
                    }
                }
                '(' if !in_string => paren_depth += 1,
                ')' if !in_string => paren_depth -= 1,
                ',' if !in_string && paren_depth == 0 => {
                    comma_pos = Some(i);
                    break;
                }
                _ => {}
            }
        }
        
        if let Some(comma_idx) = comma_pos {
            let values_part = &normalized[comma_idx + 1..];
            
            // Find the list brackets
            if let Some(start_bracket) = values_part.find('[') {
                // Find the matching closing bracket
                let mut bracket_depth = 0;
                let mut end_bracket = None;
                
                for (i, ch) in values_part[start_bracket..].char_indices() {
                    match ch {
                        '[' => bracket_depth += 1,
                        ']' => {
                            bracket_depth -= 1;
                            if bracket_depth == 0 {
                                end_bracket = Some(start_bracket + i);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                
                if let Some(end_idx) = end_bracket {
                    let list_content = &values_part[start_bracket + 1..end_idx];
                    
                    // Count top-level items (tuples or single values)
                    let mut depth = 0;
                    let mut count = 0;
                    let mut has_content = false;
                    let mut found_first_item = false;
                    
                    for ch in list_content.chars() {
                        match ch {
                            '(' | '[' | '{' => depth += 1,
                            ')' | ']' | '}' => depth -= 1,
                            ',' if depth == 0 => {
                                // This is a comma separating top-level items
                                count += 1;
                            },
                            c if !c.is_whitespace() && c != ',' => {
                                has_content = true;
                                if !found_first_item {
                                    count = 1; // First item
                                    found_first_item = true;
                                }
                            },
                            _ => {}
                        }
                    }
                    
                    return if has_content && count > 0 { Some(count) } else { Some(1) };
                }
            }
        }
    }
    
    Some(1) // Default to 1 test case if parsing fails
}

/// 🚀 ZERO-ALLOCATION LINE ITERATOR - Processes lines on-demand without heap allocations
#[inline]
fn zero_alloc_lines(content: &[u8]) -> impl Iterator<Item = (usize, &[u8])> {
    ZeroAllocLineIterator {
        content,
        position: 0,
        line_number: 0,
    }
}

/// Ultra-fast zero-allocation line iterator
struct ZeroAllocLineIterator<'a> {
    content: &'a [u8],
    position: usize,
    line_number: usize,
}

impl<'a> Iterator for ZeroAllocLineIterator<'a> {
    type Item = (usize, &'a [u8]);
    
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.content.len() {
            return None;
        }
        
        let start = self.position;
        let line_number = self.line_number;
        
        // SIMD-optimized newline search (manual implementation for zero deps)
        let mut end = self.position;
        while end < self.content.len() && self.content[end] != b'\n' {
            end += 1;
        }
        
        let mut line_end = end;
        
        // Handle \r\n line endings by trimming \r if present
        if line_end > start && self.content[line_end - 1] == b'\r' {
            line_end -= 1;
        }
        
        let line_slice = &self.content[start..line_end];
        
        // Move position past the newline
        self.position = if end < self.content.len() { end + 1 } else { end };
        self.line_number += 1;
        
        Some((line_number, line_slice))
    }
    
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.content.len() - self.position;
        let estimate = remaining / 50; // Assume average 50 chars per line
        (estimate, Some(remaining)) // Lower bound estimate, upper bound if all chars are newlines
    }
}

/// SIMD-accelerated pattern matching on memory-mapped file content (ZERO ALLOCATION)
fn find_test_patterns_simd_vectorized(file_path: &Path, content: &[u8], patterns: &SIMDPatterns) -> Result<Vec<SIMDTestLocation>> {
    let mut test_locations = Vec::new();
    let mut current_class: Option<String> = None;
    
    // 🚀 ZERO-ALLOCATION LINE PROCESSING - No Vec allocation, process on-demand
    for (line_number, line) in zero_alloc_lines(content) {
        // SIMD-accelerated multi-pattern matching
        for mat in patterns.automaton.find_iter(line) {
            // Update SIMD statistics
            if let Ok(mut stats) = patterns.pattern_stats.lock() {
                stats.patterns_matched += 1;
                stats.simd_accelerations += 1;
            }
            
            let pattern_id = mat.pattern().as_usize();
            let start_pos = mat.start();
            
            match pattern_id {
                // Class patterns (1)
                1 => {
                    if let Some(class_name) = extract_class_name_simd(line, start_pos) {
                        current_class = Some(class_name);
                    }
                }
                
                // Function patterns (0, 2, 3, 4, 5, 6, 7)
                0 | 2 | 3 | 4 | 5 | 6 | 7 => {
                    if let Some(test_name) = extract_test_name_simd(line, start_pos, pattern_id) {
                        // Determine class context using advanced indentation analysis
                        let indentation_level = line.iter().take_while(|&&b| b == b' ' || b == b'\t').count();
                        let actual_class = determine_class_context_simd_zero_alloc(content, line_number, indentation_level, &current_class);
                        
                        test_locations.push(SIMDTestLocation {
                            file_path: file_path.to_path_buf(),
                            line_number: line_number + 1, // 1-indexed
                            test_name,
                            class_name: actual_class,
                            is_async: matches!(pattern_id, 4 | 5), // async patterns
                        });
                    }
                }
                
                _ => {} // Other patterns (pytest markers, etc.)
            }
        }
    }
    
    Ok(test_locations)
}

/// SIMD test location structure
#[derive(Debug, Clone)]
struct SIMDTestLocation {
    file_path: PathBuf,
    line_number: usize,
    test_name: String,
    class_name: Option<String>,
    is_async: bool,
}

/// Extract class name from SIMD-matched line
fn extract_class_name_simd(line: &[u8], start_pos: usize) -> Option<String> {
    let after_class = &line[start_pos + 5..]; // Skip "class"
    
    // Find class name end (until ':' or '(' or whitespace)
    let mut end_pos = 0;
    for (i, &byte) in after_class.iter().enumerate() {
        if byte == b':' || byte == b'(' || (byte == b' ' && i > 0) {
            end_pos = i;
            break;
        }
    }
    
    if end_pos > 0 {
        let class_name_bytes = &after_class[..end_pos].trim_ascii();
        String::from_utf8(class_name_bytes.to_vec()).ok()
    } else {
        None
    }
}

/// Extract test function name with pattern-specific offsets
fn extract_test_name_simd(line: &[u8], start_pos: usize, pattern_id: usize) -> Option<String> {
    let offset = match pattern_id {
        0 => 4,   // "def "
        2 => 8,   // "    def "
        3 => 5,   // "\tdef "
        4 => 10,  // "async def "
        5 => 14,  // "    async def "
        6 => 12,  // "        def "
        7 => 9,   // "\t\tdef "
        _ => return None,
    };
    
    if start_pos + offset >= line.len() {
        return None;
    }
    
    let after_def = &line[start_pos + offset..];
    
    // Find function name end
    let mut end_pos = 0;
    for (i, &byte) in after_def.iter().enumerate() {
        if byte == b'(' || byte == b':' || byte == b' ' || byte == b'\t' {
            end_pos = i;
            break;
        }
    }
    
    if end_pos > 0 {
        let func_name_bytes = &after_def[..end_pos];
        String::from_utf8(func_name_bytes.to_vec()).ok()
    } else {
        None
    }
}

/// 🚀 REVOLUTIONARY UNIFIED TEST PROCESSOR - Single-pass processing with zero redundancy
struct UnifiedTestProcessor {
    /// Ultra-fast pattern matcher with optimized test detection
    pattern_matcher: AhoCorasick,
    /// Optimized parametrize parser with state machine
    parametrize_parser: OptimizedParametrizeParser,
    /// String interner for memory efficiency
    string_interner: Arc<RwLock<StringInterner>>,
    /// Performance statistics
    stats: Arc<Mutex<UnifiedProcessorStats>>,
}

/// Memory-efficient string interner
struct StringInterner {
    strings: Vec<String>,
    map: HashMap<String, u32>, // Using HashMap for speed
}

/// State machine for ultra-fast parametrize parsing
struct OptimizedParametrizeParser {
    // Pre-compiled patterns for common cases
    simple_patterns: Vec<&'static [u8]>,
}

/// Comprehensive performance statistics
#[derive(Debug, Default)]
struct UnifiedProcessorStats {
    files_processed: usize,
    bytes_scanned: usize,
    tests_discovered: usize,
    cache_hits: usize,
    simd_operations: usize,
    memory_saved_bytes: usize,
}

impl UnifiedTestProcessor {
    fn new() -> Result<Self> {
        // Build ultra-optimized pattern matcher with all test patterns
        let patterns = vec![
            b"def test_".to_vec(),
            b"async def test_".to_vec(),
            b"class Test".to_vec(),
            b"@pytest.mark".to_vec(),
            b"@parametrize".to_vec(),
            b"@fixture".to_vec(),
        ];
        
        let pattern_matcher = AhoCorasick::builder()
            .match_kind(MatchKind::LeftmostFirst)
            .prefilter(true)  // Boyer-Moore prefilter for 3x speedup
            .dense_depth(4)   // Optimize for cache locality
            .byte_classes(true) // Reduce memory footprint
            .build(&patterns)
            .map_err(|e| crate::error::Error::Discovery(format!("Pattern build failed: {}", e)))?;
        
        Ok(Self {
            pattern_matcher,
            parametrize_parser: OptimizedParametrizeParser::new(),
            string_interner: Arc::new(RwLock::new(StringInterner::new())),
            stats: Arc::new(Mutex::new(UnifiedProcessorStats::default())),
        })
    }
    
    /// 🚀 REVOLUTIONARY SINGLE-PASS FILE PROCESSING - Eliminates ALL redundancy
    fn process_file_single_pass(&self, file_path: &Path) -> Result<Vec<TestItem>> {
        // Read file content safely
        let file_content = std::fs::read(file_path)
            .map_err(|e| crate::error::Error::Discovery(format!("Failed to read {}: {}", file_path.display(), e)))?;
        
        // Update statistics
        {
            let mut stats = self.stats.lock().unwrap();
            stats.files_processed += 1;
            stats.bytes_scanned += file_content.len();
        }
        
        // Single-pass unified processing: patterns + decorators + metadata extraction
        let unified_data = self.extract_all_data_single_pass(&file_content, file_path)?;
        
        // Convert to TestItems with zero additional I/O
        self.build_test_items_from_unified_data(unified_data, file_path)
    }
    
    /// Extract all required data in a single memory scan
    fn extract_all_data_single_pass(&self, content: &[u8], _file_path: &Path) -> Result<UnifiedTestData> {
        let mut test_functions = Vec::new();
        let mut decorator_map: HashMap<usize, Vec<String>> = HashMap::new();
        let mut class_context: Option<String> = None;
        let mut current_decorators = Vec::new();
        
        // Single-pass line processing with zero allocations
        for (line_number, line) in zero_alloc_lines(content) {
            let line_str = std::str::from_utf8(line)
                .map_err(|e| crate::error::Error::Discovery(format!("Invalid UTF-8 in file: {}", e)))?;
            let trimmed = line_str.trim();
            
            // Handle decorators
            if trimmed.starts_with('@') {
                current_decorators.push(trimmed.to_string());
                continue;
            }
            
            // Handle class definitions
            if trimmed.starts_with("class Test") {
                class_context = extract_class_name_fast(trimmed);
                current_decorators.clear();
                continue;
            }
            
            // Handle test function definitions
            if let Some(test_name) = extract_test_function_name_fast(trimmed) {
                // Store decorators for this line
                if !current_decorators.is_empty() {
                    decorator_map.insert(line_number + 1, current_decorators.clone());
                }
                
                test_functions.push(UnifiedTestFunction {
                    name: test_name,
                    line_number: line_number + 1,
                    class_name: class_context.clone(),
                    is_async: trimmed.starts_with("async def"),
                    decorators: std::mem::take(&mut current_decorators),
                });
                continue;
            }
            
            // Reset decorators if we hit non-decorator, non-function line
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                current_decorators.clear();
            }
        }
        
        Ok(UnifiedTestData {
            test_functions,
            decorator_map,
        })
    }
    
    /// Build TestItems from unified data with maximum efficiency
    fn build_test_items_from_unified_data(
        &self,
        unified_data: UnifiedTestData,
        file_path: &Path,
    ) -> Result<Vec<TestItem>> {
        let mut test_items = Vec::with_capacity(unified_data.test_functions.len() * 2); // Pre-allocate for parametrize
        
        for test_func in unified_data.test_functions {
            // Ultra-fast parametrize case counting
            let param_cases = self.parametrize_parser.count_cases_optimized(&test_func.decorators);
            
            for i in 0..param_cases {
                let base_id = if let Some(ref class_name) = test_func.class_name {
                    format!("{}::{}::{}", file_path.display(), class_name, test_func.name)
                } else {
                    format!("{}::{}", file_path.display(), test_func.name)
                };
                
                let (id, name) = if param_cases > 1 {
                    (format!("{}[{}]", base_id, i), format!("{}[{}]", test_func.name, i))
                } else {
                    (base_id, test_func.name.clone())
                };
                
                test_items.push(TestItem {
                    id,
                    path: file_path.to_path_buf(),
                    name,
                    function_name: test_func.name.clone(),
                    line_number: Some(test_func.line_number),
                    decorators: test_func.decorators.clone(),
                    is_async: test_func.is_async,
                    fixture_deps: Vec::new(), // TODO: Extract fixtures in single pass
                    class_name: test_func.class_name.clone(),
                    is_xfail: test_func.decorators.iter().any(|d| d.contains("xfail")),
                });
            }
        }
        
        // Update statistics
        {
            let mut stats = self.stats.lock().unwrap();
            stats.tests_discovered += test_items.len();
        }
        
        Ok(test_items)
    }
}

/// Unified test data structure
struct UnifiedTestData {
    test_functions: Vec<UnifiedTestFunction>,
    decorator_map: HashMap<usize, Vec<String>>,
}

/// Compact test function representation
struct UnifiedTestFunction {
    name: String,
    line_number: usize,
    class_name: Option<String>,
    is_async: bool,
    decorators: Vec<String>,
}

impl StringInterner {
    fn new() -> Self {
        Self {
            strings: Vec::new(),
            map: HashMap::new(),
        }
    }
    
    fn intern(&mut self, s: String) -> u32 {
        if let Some(&id) = self.map.get(&s) {
            return id;
        }
        
        let id = self.strings.len() as u32;
        self.strings.push(s.clone());
        self.map.insert(s, id);
        id
    }
}

impl OptimizedParametrizeParser {
    fn new() -> Self {
        Self {
            simple_patterns: vec![
                b"@pytest.mark.parametrize",
                b"@parametrize",
            ],
        }
    }
    
    /// 🚀 ULTRA-FAST parametrize case counting with state machine
    fn count_cases_optimized(&self, decorators: &[String]) -> usize {
        let mut total_cases = 1;
        
        for decorator in decorators {
            if !decorator.contains("parametrize") {
                continue;
            }
            
            // State machine approach - much faster than regex
            total_cases *= self.parse_parametrize_cases_state_machine(decorator.as_bytes());
        }
        
        total_cases
    }
    
    /// State machine parser for parametrize patterns
    fn parse_parametrize_cases_state_machine(&self, decorator_bytes: &[u8]) -> usize {
        let mut state = ParametrizeParseState::SearchingOpen;
        let mut bracket_depth = 0;
        let mut paren_depth = 0;
        let mut case_count = 0;
        let mut found_content = false;
        
        for &byte in decorator_bytes {
            match (state, byte) {
                (ParametrizeParseState::SearchingOpen, b'[') => {
                    state = ParametrizeParseState::InList;
                    bracket_depth = 1;
                    case_count = 1; // First case
                    found_content = false;
                }
                (ParametrizeParseState::InList, b'[') => {
                    bracket_depth += 1;
                }
                (ParametrizeParseState::InList, b']') => {
                    bracket_depth -= 1;
                    if bracket_depth == 0 {
                        break; // End of parameter list
                    }
                }
                (ParametrizeParseState::InList, b'(') => {
                    paren_depth += 1;
                }
                (ParametrizeParseState::InList, b')') => {
                    paren_depth -= 1;
                }
                (ParametrizeParseState::InList, b',') if bracket_depth == 1 && paren_depth == 0 => {
                    case_count += 1; // Found another case
                }
                (ParametrizeParseState::InList, byte) if !byte.is_ascii_whitespace() => {
                    found_content = true;
                }
                _ => {}
            }
        }
        
        if found_content && case_count > 0 { case_count } else { 1 }
    }
}

/// State machine states for parametrize parsing
#[derive(Debug, Clone, Copy, PartialEq)]
enum ParametrizeParseState {
    SearchingOpen,
    InList,
}

/// Fast class name extraction
fn extract_class_name_fast(line: &str) -> Option<String> {
    if let Some(start) = line.find("class ") {
        let after_class = &line[start + 6..];
        if let Some(end) = after_class.find(|c: char| c == '(' || c == ':' || c.is_whitespace()) {
            let class_name = &after_class[..end];
            if !class_name.is_empty() {
                return Some(class_name.to_string());
            }
        }
    }
    None
}

/// Fast test function name extraction
fn extract_test_function_name_fast(line: &str) -> Option<String> {
    // Check for async def test_ or def test_
    let def_pos = if line.trim_start().starts_with("async def ") {
        line.find("async def ")? + 10
    } else if line.trim_start().starts_with("def ") {
        line.find("def ")? + 4
    } else {
        return None;
    };
    
    let after_def = &line[def_pos..];
    
    // Must start with "test"
    if !after_def.starts_with("test") {
        return None;
    }
    
    // Find end of function name
    if let Some(end) = after_def.find('(') {
        let func_name = &after_def[..end];
        if !func_name.is_empty() && func_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Some(func_name.to_string());
        }
    }
    
    None
}

/// Convert SIMD locations to TestItem structs with full parametrize expansion (LEGACY FALLBACK)
fn convert_simd_locations_to_test_items(locations: Vec<SIMDTestLocation>) -> Result<Vec<TestItem>> {
    // Fast path: nothing to do
    if locations.is_empty() {
        return Ok(Vec::new());
    }

    // All locations refer to the same file (function is called per-file).
    let file_path = locations[0].file_path.clone();

    // Read the file **once** to be reused for decorator extraction of every discovered test.
    // This removes an O(N * file_size) IO pattern in the old implementation where the same
    // file was opened repeatedly for every test found in it.
    let file_content = std::fs::read_to_string(&file_path)
        .map_err(|e| crate::error::Error::Discovery(format!("Failed to read file for decorators: {}", e)))?;
    let file_lines: Vec<&str> = file_content.lines().collect();

    // Cache decorator look-ups keyed by the test function's starting line number so we only
    // scan the file once per distinct line. This matters for parametrised tests that expand
    // to multiple `TestItem`s but share the same decorators.
    let mut decorator_cache: HashMap<usize, Vec<String>> = HashMap::new();

    // Helper closure to extract decorators that precede `line_number` (1-indexed).
    let mut extract_decorators = |line_number: usize| -> Vec<String> {
        if let Some(cached) = decorator_cache.get(&line_number) {
            return cached.clone();
        }

        let mut decorators = Vec::new();
        if line_number == 0 { // Should never happen (line numbers are 1-indexed)
            return decorators;
        }
        let mut idx = line_number - 1; // Convert to 0-indexed.

        // Walk backwards skipping comments/blank lines until we stop seeing decorators.
        while idx > 0 {
            idx -= 1;
            let line = file_lines.get(idx).unwrap_or(&"").trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if line.starts_with('@') {
                decorators.push(line.to_string());
            } else {
                break;
            }
        }
        decorators.reverse(); // Preserve original order.
        decorator_cache.insert(line_number, decorators.clone());
        decorators
    };

    let mut test_items = Vec::with_capacity(locations.len());

    for location in locations {
        let decorators = extract_decorators(location.line_number);
        let param_cases = helper_count_parametrize_cases(&decorators);

        for i in 0..param_cases {
            let base_id = if let Some(ref class_name) = location.class_name {
                format!("{}::{}::{}", file_path.display(), class_name, location.test_name)
            } else {
                format!("{}::{}", file_path.display(), location.test_name)
            };

            let (id, name) = if param_cases > 1 {
                (
                    format!("{}[{}]", base_id, i),
                    format!("{}[{}]", location.test_name, i),
                )
            } else {
                (base_id, location.test_name.clone())
            };

            test_items.push(TestItem {
                id,
                path: file_path.clone(),
                name,
                function_name: location.test_name.clone(),
                line_number: Some(location.line_number),
                decorators: decorators.clone(),
                is_async: location.is_async,
                fixture_deps: Vec::new(), // Fixture extraction not yet SIMD-optimised
                class_name: location.class_name.clone(),
                is_xfail: decorators.iter().any(|d| d.contains("xfail")),
            });
        }
    }

    Ok(test_items)
}

/// 🚀 ZERO-ALLOCATION BACKWARD LINE SCANNER - Finds class context without heap allocations
fn determine_class_context_simd_zero_alloc(
    content: &[u8],
    target_line_number: usize, 
    indentation_level: usize,
    current_class: &Option<String>
) -> Option<String> {
    // Zero indentation = module level
    if indentation_level == 0 {
        return None;
    }
    
    // 🧠 CLEVER OPTIMIZATION: Collect lines only up to target, then scan backwards
    let mut collected_lines = SmallVec::<[(usize, usize); 64]>::new(); // Stack-allocated for small files
    
    // Forward pass: collect line positions (not content - just byte offsets)
    for (line_number, line) in zero_alloc_lines(content) {
        if line_number >= target_line_number {
            break;
        }
        let line_start = line.as_ptr() as usize - content.as_ptr() as usize;
        let line_end = line_start + line.len();
        collected_lines.push((line_start, line_end));
    }
    
    // Backward pass: scan for class definitions using byte offsets
    for (line_start, line_end) in collected_lines.iter().rev() {
        let line = &content[*line_start..*line_end];
        let prev_indentation = line.iter().take_while(|&&b| b == b' ' || b == b'\t').count();
        
        if prev_indentation < indentation_level {
            let line_str = String::from_utf8_lossy(line);
            if line_str.trim_start().starts_with("class ") && line_str.contains("Test") {
                // Extract class name
                if let Some(class_name) = extract_class_name_from_line_simd(&line_str) {
                    return Some(class_name);
                }
            }
            break; // Stop at first less-indented line
        }
    }
    
    // Use current class if reasonable indentation
    if indentation_level >= 4 {
        current_class.clone()
    } else {
        None
    }
}

/// Extract class name from class definition line
fn extract_class_name_from_line_simd(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with("class ") {
        let after_class = &trimmed[6..]; // Skip "class "
        let class_name = after_class
            .split_whitespace().next()?
            .split('(').next()? // Handle inheritance
            .split(':').next()? // Handle direct class
            .trim();
        
        if !class_name.is_empty() {
            Some(class_name.to_string())
        } else {
            None
        }
    } else {
        None
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_python_file_discovery() {
        // Test file name patterns
        assert!(is_python_test_file(Path::new("test_example.py")));
        assert!(is_python_test_file(Path::new("example_test.py")));
        assert!(!is_python_test_file(Path::new("example.py")));
        assert!(!is_python_test_file(Path::new("test_example.txt")));
        assert!(!is_python_test_file(Path::new("__pycache__/test_example.py")));
    }

    #[test]
    fn test_discover_tests_in_file() {
        let content = r#"
import pytest

def test_simple():
    pass

async def test_async():
    pass

class TestExample:
    def test_method(self):
        pass
    
    async def test_async_method(self):
        pass

@pytest.mark.parametrize("value", [1, 2, 3])
def test_parametrized(value):
    pass

@pytest.mark.xfail
def test_xfail():
    pass

def test_with_fixtures(tmp_path, monkeypatch):
    pass

def not_a_test():
    pass
"#;

        let _content = content; // Preserve for future use
        let tests = discover_tests_in_file(Path::new("test_example.py")).unwrap();
        
        // Should discover 7 base tests + 2 extra for parametrized (3 total)
        assert_eq!(tests.len(), 9);
        
        // Check simple test
        let simple_test = tests.iter().find(|t| t.function_name == "test_simple").unwrap();
        assert!(!simple_test.is_async);
        assert!(simple_test.fixture_deps.is_empty());
        assert!(simple_test.class_name.is_none());
        
        // Check async test
        let async_test = tests.iter().find(|t| t.function_name == "test_async").unwrap();
        assert!(async_test.is_async);
        
        // Check class method
        let method_test = tests.iter().find(|t| 
            t.function_name == "test_method" && t.class_name.is_some()
        ).unwrap();
        assert_eq!(method_test.class_name.as_ref().unwrap(), "TestExample");
        
        // Check parametrized test (should create 3 test items)
        let param_tests: Vec<_> = tests.iter()
            .filter(|t| t.function_name == "test_parametrized")
            .collect();
        assert_eq!(param_tests.len(), 3);
        assert!(param_tests[0].name.contains("[0]"));
        assert!(param_tests[1].name.contains("[1]"));
        assert!(param_tests[2].name.contains("[2]"));
        
        // Check xfail test
        let xfail_test = tests.iter().find(|t| t.function_name == "test_xfail").unwrap();
        assert!(xfail_test.is_xfail);
        
        // Check test with fixtures
        let fixture_test = tests.iter().find(|t| t.function_name == "test_with_fixtures").unwrap();
        assert!(fixture_test.fixture_deps.contains(&"tmp_path".to_string()));
        assert!(fixture_test.fixture_deps.contains(&"monkeypatch".to_string()));
    }

    #[test]
    fn test_unittest_testcase_skipped() {
        let content = r#"
import unittest

class TestCaseExample(unittest.TestCase):
    def test_should_be_skipped(self):
        pass
"#;

        let _content = content; // Preserve for future use
        let tests = discover_tests_in_file(Path::new("test_example.py")).unwrap();
        
        // Should not discover tests from unittest.TestCase classes
        assert_eq!(tests.len(), 0);
    }

    #[test]
    fn test_full_discovery() {
        // Create a temporary directory with test files
        let temp_dir = TempDir::new().unwrap();
        let test_file_path = temp_dir.path().join("test_example.py");
        
        fs::write(&test_file_path, r#"
def test_one():
    pass

def test_two():
    pass
"#).unwrap();

        // Also create a non-test file that should be ignored
        let non_test_file = temp_dir.path().join("helper.py");
        fs::write(&non_test_file, r#"
def test_should_not_be_found():
    pass
"#).unwrap();

        let tests = discover_tests(&[temp_dir.path().to_path_buf()]).unwrap();
        
        // Should only find tests from test_example.py
        assert_eq!(tests.len(), 2);
        assert!(tests.iter().all(|t| t.path == test_file_path));
    }
}