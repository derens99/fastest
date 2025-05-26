use crate::cache::DiscoveryCache;
use crate::error::Result;
use crate::fixtures::{Fixture, FixtureScope};
use crate::parser::{parse_fixtures_and_tests, AstParser, FixtureDefinition, TestFunction};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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
    pub fixture_deps: Vec<String>, // Fixtures required by this test
}

pub struct DiscoveryResult {
    pub tests: Vec<TestItem>,
    pub fixtures: Vec<Fixture>,
}

pub fn discover_tests(path: &Path) -> Result<Vec<TestItem>> {
    let result = discover_tests_and_fixtures(path)?;
    Ok(result.tests)
}

pub fn discover_tests_and_fixtures(path: &Path) -> Result<DiscoveryResult> {
    let mut tests = Vec::new();
    let mut fixtures = Vec::new();

    for entry in WalkDir::new(path) {
        let entry = entry?;
        let path = entry.path();

        if is_test_file(path) {
            let content = std::fs::read_to_string(path)?;
            match parse_fixtures_and_tests(path) {
                Ok((file_fixtures, test_functions)) => {
                    // Convert fixtures
                    for fixture_def in file_fixtures {
                        fixtures.push(Fixture {
                            name: fixture_def.name.clone(),
                            scope: FixtureScope::from(fixture_def.scope.as_str()),
                            autouse: fixture_def.autouse,
                            params: vec![], // TODO: Parse params properly
                            func_path: path.to_path_buf(),
                            dependencies: extract_fixture_dependencies(&fixture_def, &content),
                        });
                    }

                    // Convert tests
                    for func in test_functions {
                        let fixture_deps = crate::fixtures::extract_fixture_deps(&func, &content);
                        tests.push(create_test_item(path, &func, fixture_deps));
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
                }
            }
        }
    }

    Ok(DiscoveryResult { tests, fixtures })
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
                        let fixture_deps = crate::fixtures::extract_fixture_deps(&func, &content);
                        tests.push(create_test_item(path, &func, fixture_deps));
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
            let content = std::fs::read_to_string(path)?;
            match parse_fixtures_and_tests(path) {
                Ok((_, test_functions)) => {
                    let file_tests: Vec<TestItem> = test_functions
                        .iter()
                        .map(|func| {
                            let fixture_deps =
                                crate::fixtures::extract_fixture_deps(func, &content);
                            create_test_item(path, func, fixture_deps)
                        })
                        .collect();

                    // Update cache
                    if let Err(e) = cache.update(path.to_path_buf(), file_tests.clone()) {
                        eprintln!(
                            "Warning: Failed to update cache for {}: {}",
                            path.display(),
                            e
                        );
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
        eprintln!(
            "Discovery cache: {} hits, {} misses",
            cache_hits, cache_misses
        );
    }

    Ok(tests)
}

fn is_test_file(path: &Path) -> bool {
    path.is_file()
        && path.extension().map_or(false, |ext| ext == "py")
        && (path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("test_")
            || path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .ends_with("_test.py"))
}

fn create_test_item(path: &Path, func: &TestFunction, fixture_deps: Vec<String>) -> TestItem {
    let module_path = path
        .with_extension("")
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
        fixture_deps,
    }
}

fn extract_fixture_dependencies(fixture: &FixtureDefinition, content: &str) -> Vec<String> {
    // Extract fixture dependencies from its parameters
    let lines: Vec<&str> = content.lines().collect();
    if fixture.line_number > 0 && fixture.line_number <= lines.len() {
        let func_line = lines[fixture.line_number - 1];

        // Extract parameters from function signature
        if let Some(start) = func_line.find('(') {
            if let Some(end) = func_line.find(')') {
                let params_str = &func_line[start + 1..end];
                let deps: Vec<String> = params_str
                    .split(',')
                    .map(|p| p.trim())
                    .filter(|p| !p.is_empty() && *p != "request") // 'request' is a special fixture
                    .map(|p| {
                        // Handle type annotations
                        p.split(':').next().unwrap_or(p).trim().to_string()
                    })
                    .collect();
                return deps;
            }
        }
    }

    Vec::new()
}
