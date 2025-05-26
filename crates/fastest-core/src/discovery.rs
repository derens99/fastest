use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::error::Result;
use crate::parser::{parse_test_file, TestFunction, AstParser};
use crate::cache::DiscoveryCache;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TestItem {
    pub id: String,
    pub path: PathBuf,
    pub name: String,
    pub function_name: String,
    pub line_number: usize,
    pub is_async: bool,
    pub class_name: Option<String>,
    pub decorators: Vec<String>,
}

pub fn discover_tests(path: &Path) -> Result<Vec<TestItem>> {
    let mut tests = Vec::new();
    
    for entry in WalkDir::new(path) {
        let entry = entry?;
        let path = entry.path();
        
        if is_test_file(path) {
            match parse_test_file(path) {
                Ok(test_functions) => {
                    for func in test_functions {
                        tests.push(create_test_item(path, &func));
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
                }
            }
        }
    }
    
    Ok(tests)
}

/// Discover tests using the AST parser
pub fn discover_tests_ast(path: &Path) -> Result<Vec<TestItem>> {
    let mut tests = Vec::new();
    let mut parser = AstParser::new()?;
    
    for entry in WalkDir::new(path) {
        let entry = entry?;
        let path = entry.path();
        
        if is_test_file(path) {
            let content = std::fs::read_to_string(path)?;
            match parser.parse_file(&content, path.to_str().unwrap_or("")) {
                Ok(test_functions) => {
                    for func in test_functions {
                        tests.push(create_test_item(path, &func));
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
                }
            }
        }
    }
    
    Ok(tests)
}

/// Discover tests with caching support
pub fn discover_tests_cached(path: &Path, cache: &mut DiscoveryCache) -> Result<Vec<TestItem>> {
    let mut tests = Vec::new();
    let mut cache_hits = 0;
    let mut cache_misses = 0;
    
    for entry in WalkDir::new(path) {
        let entry = entry?;
        let path = entry.path();
        
        if is_test_file(path) {
            // Try to get from cache first
            if let Some(cached_tests) = cache.get(path) {
                tests.extend(cached_tests.clone());
                cache_hits += 1;
                continue;
            }
            
            // Cache miss - parse the file
            cache_misses += 1;
            match parse_test_file(path) {
                Ok(test_functions) => {
                    let file_tests: Vec<TestItem> = test_functions
                        .iter()
                        .map(|func| create_test_item(path, func))
                        .collect();
                    
                    // Update cache
                    if let Err(e) = cache.update(path.to_path_buf(), file_tests.clone()) {
                        eprintln!("Warning: Failed to update cache for {}: {}", path.display(), e);
                    }
                    
                    tests.extend(file_tests);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
                }
            }
        }
    }
    
    if cache_hits > 0 || cache_misses > 0 {
        eprintln!("Discovery cache: {} hits, {} misses", cache_hits, cache_misses);
    }
    
    Ok(tests)
}

fn is_test_file(path: &Path) -> bool {
    path.is_file() 
        && path.extension().map_or(false, |ext| ext == "py")
        && (path.file_name().unwrap().to_str().unwrap().starts_with("test_")
            || path.file_name().unwrap().to_str().unwrap().ends_with("_test.py"))
}

fn create_test_item(path: &Path, func: &TestFunction) -> TestItem {
    let module_path = path.with_extension("")
        .to_string_lossy()
        .replace('/', ".")
        .replace('\\', ".");
    
    let test_id = if let Some(class) = &func.class_name {
        format!("{}::{}::{}", module_path, class, func.name)
    } else {
        format!("{}::{}", module_path, func.name)
    };
    
    TestItem {
        id: test_id,
        path: path.to_path_buf(),
        name: format!("{} (line {})", func.name, func.line_number),
        function_name: func.name.clone(),
        line_number: func.line_number,
        is_async: func.is_async,
        class_name: func.class_name.clone(),
        decorators: func.decorators.clone(),
    }
}