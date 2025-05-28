use crate::cache::DiscoveryCache;
use crate::error::Result;
use crate::fixtures::{Fixture, FixtureScope};
use crate::parametrize::expand_parametrized_tests;
use crate::parser::{FixtureDefinition, Parser, TestFunction};
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
    pub is_xfail: bool,            // Whether the test is expected to fail
}

pub struct DiscoveryResult {
    pub tests: Vec<TestItem>,
    pub fixtures: Vec<Fixture>,
}

/// Check if a fixture is applicable to a test based on scope
fn is_fixture_applicable_to_test(fixture: &Fixture, test: &TestItem) -> bool {
    match fixture.scope {
        FixtureScope::Session => true, // Session fixtures apply to all tests
        FixtureScope::Module => {
            // Module fixtures apply to tests in the same module
            let fixture_module = fixture
                .func_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            let test_module = test.path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            fixture_module == test_module
        }
        FixtureScope::Class => {
            // Class fixtures only apply if defined in the same class
            // For now, we can't determine class membership at this level
            // So we don't add class autouse fixtures automatically
            // They need to be handled in the execution phase
            false
        }
        FixtureScope::Function => {
            // Function-scoped autouse fixtures apply to all tests in the same file
            fixture.func_path == test.path
        }
    }
}

pub fn discover_tests(path: &Path) -> Result<Vec<TestItem>> {
    let result = discover_tests_and_fixtures(path)?;
    // The autouse fixtures have already been added to tests in discover_tests_and_fixtures
    Ok(result.tests)
}

pub fn discover_tests_and_fixtures(path: &Path) -> Result<DiscoveryResult> {
    let mut tests = Vec::new();
    let mut fixtures = Vec::new();

    // Skip common virtual environment and build directories
    let excluded_dirs = vec![
        ".venv",
        "venv",
        "env",
        ".env",
        "virtualenv",
        ".virtualenv",
        "__pycache__",
        ".git",
        ".tox",
        "site-packages",
        "dist",
        "build",
        ".eggs",
        "*.egg-info",
        "node_modules",
    ];

    for entry in WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Skip excluded directories
            if e.file_type().is_dir() {
                let dir_name = e.file_name().to_string_lossy();
                // Check if it's an excluded directory
                if excluded_dirs.iter().any(|&excluded| dir_name == excluded) {
                    return false;
                }
                // Also skip if the path contains site-packages anywhere
                if e.path().to_string_lossy().contains("site-packages") {
                    return false;
                }
            }
            true
        })
    {
        let entry = entry?;
        let path = entry.path();

        if is_test_file(path) {
            let content = std::fs::read_to_string(path)?;
            match Parser::parse_fixtures_and_tests(path) {
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
                        let test_item = create_test_item(path, &func, fixture_deps);

                        // Debug: print decorators
                        if !func.decorators.is_empty() {
                            eprintln!("Test {} has decorators: {:?}", func.name, func.decorators);
                        }

                        // Expand parametrized tests
                        let mut expanded_tests_for_func =
                            expand_parametrized_tests(&test_item, &func.decorators)?;
                        // If not parametrized, expand_parametrized_tests returns the original test item in a vec.
                        // We need to set its xfail status based on top-level decorators like @pytest.mark.xfail
                        if expanded_tests_for_func.len() == 1
                            && test_item.id == expanded_tests_for_func[0].id
                        {
                            if func.decorators.iter().any(|d| d.contains("xfail")) {
                                expanded_tests_for_func[0].is_xfail = true;
                            }
                        }
                        tests.extend(expanded_tests_for_func);
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
                }
            }
        }
    }

    // Add autouse fixtures to all applicable tests
    for test in &mut tests {
        for fixture in &fixtures {
            if fixture.autouse && is_fixture_applicable_to_test(&fixture, &test) {
                // Only add if not already present
                if !test.fixture_deps.contains(&fixture.name) {
                    test.fixture_deps.push(fixture.name.clone());
                }
            }
        }
    }

    Ok(DiscoveryResult { tests, fixtures })
}

/// Discover tests using the AST parser (for backward compatibility)
pub fn discover_tests_ast(path: &Path) -> Result<Vec<TestItem>> {
    discover_tests(path)
}

/// Discover tests with caching support
pub fn discover_tests_cached(path: &Path, cache: &mut DiscoveryCache) -> Result<Vec<TestItem>> {
    let mut tests = Vec::new();
    let mut cache_hits = 0;
    let mut cache_misses = 0;

    // Skip common virtual environment and build directories
    let excluded_dirs = vec![
        ".venv",
        "venv",
        "env",
        ".env",
        "virtualenv",
        ".virtualenv",
        "__pycache__",
        ".git",
        ".tox",
        "site-packages",
        "dist",
        "build",
        ".eggs",
        "*.egg-info",
        "node_modules",
    ];

    for entry in WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Skip excluded directories
            if e.file_type().is_dir() {
                let dir_name = e.file_name().to_string_lossy();
                // Check if it's an excluded directory
                if excluded_dirs.iter().any(|&excluded| dir_name == excluded) {
                    return false;
                }
                // Also skip if the path contains site-packages anywhere
                if e.path().to_string_lossy().contains("site-packages") {
                    return false;
                }
            }
            true
        })
    {
        let entry = entry?;
        let path = entry.path();

        if is_test_file(path) {
            // Try to get from cache first
            if let Some(cached_tests) = cache.get(path) {
                tests.extend(cached_tests);
                cache_hits += 1;
                continue;
            }

            // Cache miss - parse the file
            cache_misses += 1;
            let content = std::fs::read_to_string(path)?;
            // NOTE: discover_tests_cached now only returns TestItems.
            // If full fixture discovery is needed with caching, this function needs to be refactored
            // similar to discover_tests_and_fixtures and cache DiscoveryResult or (Vec<FixtureDefinition>, Vec<TestFunction>).
            // For now, it uses the provided parser_type to parse tests.
            match Parser::parse_fixtures_and_tests(path) {
                Ok((_file_fixtures, test_functions)) => {
                    // Ignoring fixtures for now in cached version
                    let mut file_tests = Vec::new();

                    for func in test_functions {
                        let fixture_deps = crate::fixtures::extract_fixture_deps(&func, &content);
                        let test_item = create_test_item(path, &func, fixture_deps);

                        // Debug: print decorators
                        if !func.decorators.is_empty() {
                            eprintln!("Test {} has decorators: {:?}", func.name, func.decorators);
                        }

                        // Expand parametrized tests
                        let mut expanded_tests_for_func =
                            expand_parametrized_tests(&test_item, &func.decorators)?;
                        if expanded_tests_for_func.len() == 1
                            && test_item.id == expanded_tests_for_func[0].id
                        {
                            if func.decorators.iter().any(|d| d.contains("xfail")) {
                                expanded_tests_for_func[0].is_xfail = true;
                            }
                        }
                        file_tests.extend(expanded_tests_for_func);
                    }

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
        && path.extension().is_some_and(|ext| ext == "py")
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
    // Get just the filename without extension for the module path
    let module_path = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let test_id = if let Some(class) = &func.class_name {
        format!("{}::{}::{}", module_path, class, func.name)
    } else {
        format!("{}::{}", module_path, func.name)
    };

    // Debug output
    if func.class_name.is_some() {
        eprintln!(
            "DEBUG: Creating test item with class - ID: {}, class: {:?}, func: {}",
            test_id, func.class_name, func.name
        );
    }

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
        is_xfail: false, // Default to false, will be updated by parametrize or decorator parsing
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
