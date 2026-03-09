//! Discovery and extraction of fixtures from `conftest.py` files.
//!
//! Walks from a project root to find all `conftest.py` files, then parses
//! each one using `rustpython-parser` to extract functions decorated with
//! `@pytest.fixture`.

use crate::discovery::should_skip_dir;
use crate::error::{Error, Result};
use crate::fixtures::{Fixture, FixtureScope};
use rustpython_parser::ast::{self, Constant, Expr, Stmt};
use rustpython_parser::Parse;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// ---------------------------------------------------------------------------
// Hierarchical conftest fixture map (Phase 4.1)
// ---------------------------------------------------------------------------

/// A directory-scoped conftest fixture registry.
///
/// Stores fixtures keyed by the directory containing their `conftest.py`,
/// enabling proper pytest scoping where sibling directories with the same
/// fixture name get isolated values.
#[derive(Debug, Clone, Default)]
pub struct ConftestMap {
    /// `(directory, fixtures)` pairs sorted shallowest-first.
    layers: Vec<(PathBuf, HashMap<String, Fixture>)>,
}

impl ConftestMap {
    /// Discover conftest fixtures under `root`, building a hierarchical map.
    pub fn discover(root: &Path) -> Result<Self> {
        Self::discover_with_config(root, &[])
    }

    /// Discover with extra skip directories from config.
    pub fn discover_with_config(root: &Path, norecursedirs: &[String]) -> Result<Self> {
        let conftest_paths = collect_conftest_paths(root, norecursedirs);
        let mut layers = Vec::new();

        for conftest_path in &conftest_paths {
            let dir = conftest_path
                .parent()
                .unwrap_or(Path::new("."))
                .to_path_buf();

            let source = match std::fs::read_to_string(conftest_path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Warning: failed to read {}: {}", conftest_path.display(), e);
                    continue;
                }
            };

            match extract_fixtures_from_source(&source, conftest_path) {
                Ok(fixtures) if !fixtures.is_empty() => {
                    layers.push((dir, fixtures));
                }
                Ok(_) => {} // empty conftest
                Err(e) => {
                    eprintln!(
                        "Warning: failed to parse {}: {}",
                        conftest_path.display(),
                        e
                    );
                }
            }
        }

        Ok(Self { layers })
    }

    /// Resolve fixtures visible to a test at `test_path`.
    ///
    /// Walks from the test file's directory toward the root, collecting
    /// fixtures from each conftest layer whose directory is an ancestor.
    /// Closer conftest files override more distant ones.
    pub fn resolve_for_test(&self, test_path: &Path) -> HashMap<String, Fixture> {
        let test_dir = test_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let mut merged = HashMap::new();

        // layers are shallowest-first, so we iterate in order and deeper
        // entries naturally override shallower ones.
        for (dir, fixtures) in &self.layers {
            if test_dir.starts_with(dir) || dir == Path::new(".") {
                for (name, fixture) in fixtures {
                    merged.insert(name.clone(), fixture.clone());
                }
            }
        }

        merged
    }

    /// Flatten all fixtures into a single map (deeper overrides shallower).
    ///
    /// This is the legacy behaviour — use [`resolve_for_test`] for proper
    /// scoping when a test path is available.
    pub fn flatten(&self) -> HashMap<String, Fixture> {
        let mut merged = HashMap::new();
        for (_dir, fixtures) in &self.layers {
            for (name, fixture) in fixtures {
                merged.insert(name.clone(), fixture.clone());
            }
        }
        merged
    }

    /// Return all autouse fixture names across all layers.
    pub fn autouse_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for (_dir, fixtures) in &self.layers {
            for fixture in fixtures.values() {
                if fixture.autouse && !names.contains(&fixture.name) {
                    names.push(fixture.name.clone());
                }
            }
        }
        names
    }

    /// Return autouse fixture names visible to a specific test path.
    pub fn autouse_for_test(&self, test_path: &Path) -> Vec<String> {
        let resolved = self.resolve_for_test(test_path);
        resolved
            .values()
            .filter(|f| f.autouse)
            .map(|f| f.name.clone())
            .collect()
    }

    /// Check if the map is empty (no conftest fixtures at all).
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Helper: collect conftest.py paths sorted by depth
// ---------------------------------------------------------------------------

fn collect_conftest_paths(root: &Path, norecursedirs: &[String]) -> Vec<PathBuf> {
    let mut conftest_paths: Vec<PathBuf> = Vec::new();

    let walker = WalkDir::new(root).into_iter().filter_entry(|e| {
        if e.file_type().is_dir() {
            if let Some(name) = e.file_name().to_str() {
                return !should_skip_dir(name, norecursedirs);
            }
        }
        true
    });
    for entry in walker.filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            if let Some(name) = entry.file_name().to_str() {
                if name == "conftest.py" {
                    conftest_paths.push(entry.into_path());
                }
            }
        }
    }

    // Sort by path depth (component count) so shallow conftest files come first.
    // Use the path string as a secondary key for deterministic ordering of siblings.
    conftest_paths.sort_by(|a, b| {
        a.components()
            .count()
            .cmp(&b.components().count())
            .then_with(|| a.cmp(b))
    });
    conftest_paths
}

// ---------------------------------------------------------------------------
// Legacy flat API (delegates to ConftestMap)
// ---------------------------------------------------------------------------

/// Discover all fixtures defined in `conftest.py` files under `root`.
///
/// Walks the directory tree starting at `root`, finds every file named
/// `conftest.py`, parses it, and extracts all `@pytest.fixture`-decorated
/// function definitions.  Fixtures from deeper directories override those
/// from shallower ones (closer conftest wins).
pub fn discover_conftest_fixtures(root: &Path) -> Result<HashMap<String, Fixture>> {
    discover_conftest_fixtures_with_config(root, &[])
}

/// Discover conftest fixtures, respecting additional skip directories from config.
pub fn discover_conftest_fixtures_with_config(
    root: &Path,
    norecursedirs: &[String],
) -> Result<HashMap<String, Fixture>> {
    let map = ConftestMap::discover_with_config(root, norecursedirs)?;
    Ok(map.flatten())
}

/// Extract fixture definitions from a single Python source string.
///
/// Parses the source with `rustpython-parser` and looks for top-level
/// function definitions decorated with `@pytest.fixture` or `@pytest.fixture(...)`.
pub fn extract_fixtures_from_source(source: &str, path: &Path) -> Result<HashMap<String, Fixture>> {
    let stmts = ast::Suite::parse(source, &path.display().to_string())
        .map_err(|e| Error::Parse(format!("Failed to parse {}: {}", path.display(), e)))?;

    let mut fixtures = HashMap::new();

    for stmt in &stmts {
        if let Some(fixture) = try_extract_fixture(stmt, path) {
            fixtures.insert(fixture.name.clone(), fixture);
        }
    }

    Ok(fixtures)
}

/// Attempt to extract a Fixture from a single statement, if it is a
/// `@pytest.fixture`-decorated function definition.
fn try_extract_fixture(stmt: &Stmt, path: &Path) -> Option<Fixture> {
    match stmt {
        Stmt::FunctionDef(func_def) => extract_fixture_from_decorators(
            func_def.name.as_str(),
            &func_def.decorator_list,
            &func_def.args,
            &func_def.body,
            path,
        ),
        Stmt::AsyncFunctionDef(func_def) => extract_fixture_from_decorators(
            func_def.name.as_str(),
            &func_def.decorator_list,
            &func_def.args,
            &func_def.body,
            path,
        ),
        _ => None,
    }
}

/// Given a function's name, decorators, arguments, and body, check whether
/// it has a `@pytest.fixture` decorator and, if so, build a [`Fixture`].
fn extract_fixture_from_decorators(
    func_name: &str,
    decorators: &[Expr],
    args: &ast::Arguments,
    body: &[Stmt],
    path: &Path,
) -> Option<Fixture> {
    let mut scope = FixtureScope::Function;
    let mut autouse = false;
    let mut params: Vec<serde_json::Value> = Vec::new();
    let mut found = false;

    for decorator in decorators {
        if is_pytest_fixture(decorator) {
            found = true;

            // If the decorator is a Call, parse its keyword arguments
            if let Expr::Call(call) = decorator {
                for kw in &call.keywords {
                    if let Some(ref arg_name) = kw.arg {
                        match arg_name.as_str() {
                            "scope" => {
                                scope = parse_scope_from_expr(&kw.value);
                            }
                            "autouse" => {
                                autouse = parse_bool_from_expr(&kw.value);
                            }
                            "params" => {
                                params = parse_params_from_expr(&kw.value);
                            }
                            _ => {}
                        }
                    }
                }
            }
            break;
        }
    }

    if !found {
        return None;
    }

    let dependencies = extract_parameter_names(args);
    let is_yield = body_contains_yield(body);

    Some(Fixture {
        name: func_name.to_string(),
        scope,
        autouse,
        params,
        func_path: path.to_path_buf(),
        dependencies,
        is_yield,
    })
}

/// Check whether an expression is `pytest.fixture` or `pytest.fixture(...)`.
fn is_pytest_fixture(expr: &Expr) -> bool {
    match expr {
        Expr::Attribute(attr) => attr.attr.as_str() == "fixture" && is_pytest_name(&attr.value),
        Expr::Call(call) => is_pytest_fixture(&call.func),
        _ => false,
    }
}

/// Check whether an expression is the name `pytest`.
fn is_pytest_name(expr: &Expr) -> bool {
    matches!(expr, Expr::Name(name) if name.id.as_str() == "pytest")
}

/// Parse a scope string from a keyword argument value.
fn parse_scope_from_expr(expr: &Expr) -> FixtureScope {
    if let Expr::Constant(c) = expr {
        if let Constant::Str(s) = &c.value {
            return match s.as_str() {
                "function" => FixtureScope::Function,
                "class" => FixtureScope::Class,
                "module" => FixtureScope::Module,
                "package" => FixtureScope::Package,
                "session" => FixtureScope::Session,
                _ => FixtureScope::Function,
            };
        }
    }
    FixtureScope::Function
}

/// Parse a boolean from an expression (handles `True` / `False` constants and Name nodes).
fn parse_bool_from_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Constant(c) => matches!(&c.value, Constant::Bool(true)),
        Expr::Name(name) => name.id.as_str() == "True",
        _ => false,
    }
}

/// Parse a `params=[...]` list from an expression into JSON values.
fn parse_params_from_expr(expr: &Expr) -> Vec<serde_json::Value> {
    let elements = match expr {
        Expr::List(list) => &list.elts,
        Expr::Tuple(tuple) => &tuple.elts,
        _ => return Vec::new(),
    };

    elements.iter().map(expr_to_json_value).collect()
}

/// Convert an AST expression to a `serde_json::Value`.
fn expr_to_json_value(expr: &Expr) -> serde_json::Value {
    match expr {
        Expr::Constant(c) => match &c.value {
            Constant::Int(n) => {
                let s = n.to_string();
                if let Ok(i) = s.parse::<i64>() {
                    serde_json::Value::Number(i.into())
                } else {
                    serde_json::Value::String(s)
                }
            }
            Constant::Float(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Constant::Str(s) => serde_json::Value::String(s.clone()),
            Constant::Bool(b) => serde_json::Value::Bool(*b),
            Constant::None => serde_json::Value::Null,
            _ => serde_json::Value::Null,
        },
        Expr::Name(name) => match name.id.as_str() {
            "True" => serde_json::Value::Bool(true),
            "False" => serde_json::Value::Bool(false),
            "None" => serde_json::Value::Null,
            other => serde_json::Value::String(other.to_string()),
        },
        Expr::List(list) => {
            let items: Vec<serde_json::Value> = list.elts.iter().map(expr_to_json_value).collect();
            serde_json::Value::Array(items)
        }
        _ => serde_json::Value::Null,
    }
}

/// Extract parameter names from function arguments, excluding `request`.
///
/// Fixture functions receive other fixtures as arguments, so the parameter
/// names (minus `request`, which is provided by pytest itself) become
/// the fixture's dependency list.
fn extract_parameter_names(args: &ast::Arguments) -> Vec<String> {
    let all_args = args
        .posonlyargs
        .iter()
        .chain(args.args.iter())
        .chain(args.kwonlyargs.iter());

    all_args
        .map(|arg_with_default| arg_with_default.def.arg.as_str().to_string())
        .filter(|name| name != "request")
        .collect()
}

/// Check whether a function body contains a `yield` statement (indicating
/// a yield-based fixture with teardown).
fn body_contains_yield(body: &[Stmt]) -> bool {
    for stmt in body {
        match stmt {
            Stmt::Expr(expr_stmt) => {
                if contains_yield_expr(&expr_stmt.value) {
                    return true;
                }
            }
            _ => {
                // Walk nested statements (if, for, try, etc.)
                if stmt_contains_yield(stmt) {
                    return true;
                }
            }
        }
    }
    false
}

/// Recursively check whether a statement or its children contain a yield.
fn stmt_contains_yield(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Expr(expr_stmt) => contains_yield_expr(&expr_stmt.value),
        // Handle `result = yield value` — yield on RHS of assignment
        Stmt::Assign(assign) => contains_yield_expr(&assign.value),
        Stmt::AnnAssign(ann_assign) => ann_assign
            .value
            .as_ref()
            .map(|v| contains_yield_expr(v))
            .unwrap_or(false),
        Stmt::Return(ret) => ret
            .value
            .as_ref()
            .map(|v| contains_yield_expr(v))
            .unwrap_or(false),
        Stmt::If(if_stmt) => {
            body_contains_yield(&if_stmt.body) || body_contains_yield(&if_stmt.orelse)
        }
        Stmt::Try(try_stmt) => {
            body_contains_yield(&try_stmt.body)
                || body_contains_yield(&try_stmt.orelse)
                || body_contains_yield(&try_stmt.finalbody)
                || try_stmt.handlers.iter().any(|h| match h {
                    ast::ExceptHandler::ExceptHandler(eh) => body_contains_yield(&eh.body),
                })
        }
        Stmt::TryStar(try_stmt) => {
            body_contains_yield(&try_stmt.body)
                || body_contains_yield(&try_stmt.orelse)
                || body_contains_yield(&try_stmt.finalbody)
                || try_stmt.handlers.iter().any(|h| match h {
                    ast::ExceptHandler::ExceptHandler(eh) => body_contains_yield(&eh.body),
                })
        }
        Stmt::For(for_stmt) => body_contains_yield(&for_stmt.body),
        Stmt::While(while_stmt) => body_contains_yield(&while_stmt.body),
        Stmt::With(with_stmt) => body_contains_yield(&with_stmt.body),
        _ => false,
    }
}

/// Check whether an expression is a yield expression.
fn contains_yield_expr(expr: &Expr) -> bool {
    matches!(expr, Expr::Yield(_) | Expr::YieldFrom(_))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_simple_fixture() {
        let source = r#"
import pytest

@pytest.fixture
def db_connection():
    conn = create_connection()
    return conn
"#;
        let path = PathBuf::from("conftest.py");
        let fixtures = extract_fixtures_from_source(source, &path).unwrap();

        assert_eq!(fixtures.len(), 1);
        let fixture = fixtures.get("db_connection").unwrap();
        assert_eq!(fixture.name, "db_connection");
        assert_eq!(fixture.scope, FixtureScope::Function);
        assert!(!fixture.autouse);
        assert!(!fixture.is_yield);
        assert!(fixture.dependencies.is_empty());
    }

    #[test]
    fn test_extract_fixture_with_scope() {
        let source = r#"
import pytest

@pytest.fixture(scope="session")
def app():
    return create_app()
"#;
        let path = PathBuf::from("conftest.py");
        let fixtures = extract_fixtures_from_source(source, &path).unwrap();

        let fixture = fixtures.get("app").unwrap();
        assert_eq!(fixture.scope, FixtureScope::Session);
    }

    #[test]
    fn test_extract_fixture_with_autouse() {
        let source = r#"
import pytest

@pytest.fixture(autouse=True)
def setup_logging():
    import logging
    logging.basicConfig()
"#;
        let path = PathBuf::from("conftest.py");
        let fixtures = extract_fixtures_from_source(source, &path).unwrap();

        let fixture = fixtures.get("setup_logging").unwrap();
        assert!(fixture.autouse);
    }

    #[test]
    fn test_extract_yield_fixture() {
        let source = r#"
import pytest

@pytest.fixture
def db():
    conn = connect()
    yield conn
    conn.close()
"#;
        let path = PathBuf::from("conftest.py");
        let fixtures = extract_fixtures_from_source(source, &path).unwrap();

        let fixture = fixtures.get("db").unwrap();
        assert!(fixture.is_yield);
    }

    #[test]
    fn test_extract_fixture_with_dependencies() {
        let source = r#"
import pytest

@pytest.fixture
def user(db_connection, config):
    return create_user(db_connection, config)
"#;
        let path = PathBuf::from("conftest.py");
        let fixtures = extract_fixtures_from_source(source, &path).unwrap();

        let fixture = fixtures.get("user").unwrap();
        assert_eq!(fixture.dependencies, vec!["db_connection", "config"]);
    }

    #[test]
    fn test_non_fixture_functions_excluded() {
        let source = r#"
import pytest

def helper():
    pass

@pytest.fixture
def real_fixture():
    return 42

class TestSomething:
    def test_foo(self):
        pass
"#;
        let path = PathBuf::from("conftest.py");
        let fixtures = extract_fixtures_from_source(source, &path).unwrap();

        assert_eq!(fixtures.len(), 1);
        assert!(fixtures.contains_key("real_fixture"));
    }

    #[test]
    fn test_extract_fixture_with_params() {
        let source = r#"
import pytest

@pytest.fixture(params=[1, 2, 3])
def number(request):
    return request.param
"#;
        let path = PathBuf::from("conftest.py");
        let fixtures = extract_fixtures_from_source(source, &path).unwrap();

        let fixture = fixtures.get("number").unwrap();
        assert_eq!(fixture.params.len(), 3);
        assert_eq!(fixture.params[0], serde_json::json!(1));
        assert_eq!(fixture.params[1], serde_json::json!(2));
        assert_eq!(fixture.params[2], serde_json::json!(3));
        // `request` should be filtered out of dependencies
        assert!(fixture.dependencies.is_empty());
    }

    #[test]
    fn test_discover_conftest_fixtures_filesystem() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Create a conftest.py at root
        std::fs::write(
            root.join("conftest.py"),
            r#"
import pytest

@pytest.fixture
def root_fixture():
    return "root"
"#,
        )
        .unwrap();

        // Create a conftest.py in a subdirectory
        let sub = root.join("tests");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(
            sub.join("conftest.py"),
            r#"
import pytest

@pytest.fixture
def sub_fixture(root_fixture):
    return "sub"
"#,
        )
        .unwrap();

        let fixtures = discover_conftest_fixtures(root).unwrap();

        assert!(fixtures.contains_key("root_fixture"));
        assert!(fixtures.contains_key("sub_fixture"));

        let sub_fix = fixtures.get("sub_fixture").unwrap();
        assert_eq!(sub_fix.dependencies, vec!["root_fixture"]);
    }

    #[test]
    fn test_deeper_conftest_overrides() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Root conftest defines `shared`
        std::fs::write(
            root.join("conftest.py"),
            r#"
import pytest

@pytest.fixture(scope="session")
def shared():
    return "root_version"
"#,
        )
        .unwrap();

        // Subdirectory conftest redefines `shared`
        let sub = root.join("tests");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(
            sub.join("conftest.py"),
            r#"
import pytest

@pytest.fixture(scope="function")
def shared():
    return "sub_version"
"#,
        )
        .unwrap();

        let fixtures = discover_conftest_fixtures(root).unwrap();

        // The deeper definition should win
        let shared = fixtures.get("shared").unwrap();
        assert_eq!(shared.scope, FixtureScope::Function);
    }

    #[test]
    fn test_conftest_map_sibling_isolation() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Root conftest
        std::fs::write(
            root.join("conftest.py"),
            r#"
import pytest

@pytest.fixture
def root_fix():
    return "root"
"#,
        )
        .unwrap();

        // tests/a/conftest.py
        let a = root.join("tests").join("a");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::write(
            a.join("conftest.py"),
            r#"
import pytest

@pytest.fixture
def shared():
    return "a_version"
"#,
        )
        .unwrap();

        // tests/b/conftest.py with different "shared"
        let b = root.join("tests").join("b");
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(
            b.join("conftest.py"),
            r#"
import pytest

@pytest.fixture(scope="session")
def shared():
    return "b_version"
"#,
        )
        .unwrap();

        let map = ConftestMap::discover(root).unwrap();

        // Resolve for a test in tests/a → should see a's "shared"
        let a_fixtures = map.resolve_for_test(&a.join("test_a.py"));
        assert!(a_fixtures.contains_key("root_fix"));
        assert_eq!(a_fixtures["shared"].scope, FixtureScope::Function);

        // Resolve for a test in tests/b → should see b's "shared"
        let b_fixtures = map.resolve_for_test(&b.join("test_b.py"));
        assert!(b_fixtures.contains_key("root_fix"));
        assert_eq!(b_fixtures["shared"].scope, FixtureScope::Session);
    }

    #[test]
    fn test_resolve_for_test_at_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        std::fs::write(
            root.join("conftest.py"),
            r#"
import pytest

@pytest.fixture
def root_fix():
    return "root"
"#,
        )
        .unwrap();

        let map = ConftestMap::discover(root).unwrap();

        // A test file at the project root should see root fixtures
        let fixtures = map.resolve_for_test(&root.join("test_bare.py"));
        assert!(
            fixtures.contains_key("root_fix"),
            "test at root should resolve root fixtures"
        );

        // Also test bare filename resolve — empty parent gets mapped to "."
        let bare = std::path::Path::new("test_bare.py");
        let bare_dir = bare
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(std::path::Path::new("."));
        assert_eq!(bare_dir, std::path::Path::new("."));
    }

    #[test]
    fn test_autouse_for_test_sibling_isolation() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // tests/a/conftest.py with autouse fixture
        let a = root.join("tests").join("a");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::write(
            a.join("conftest.py"),
            r#"
import pytest

@pytest.fixture(autouse=True)
def a_autouse():
    pass
"#,
        )
        .unwrap();

        // tests/b/ has no conftest
        let b = root.join("tests").join("b");
        std::fs::create_dir_all(&b).unwrap();

        let map = ConftestMap::discover(root).unwrap();

        // Test in tests/a should see a_autouse
        let a_autouse = map.autouse_for_test(&a.join("test_a.py"));
        assert!(a_autouse.contains(&"a_autouse".to_string()));

        // Test in tests/b should NOT see a_autouse
        let b_autouse = map.autouse_for_test(&b.join("test_b.py"));
        assert!(
            !b_autouse.contains(&"a_autouse".to_string()),
            "sibling directory should not see a_autouse"
        );
    }

    #[test]
    fn test_yield_in_try_except_else() {
        // Yield inside except handler
        let source = r#"
import pytest

@pytest.fixture
def fix_except():
    try:
        setup()
    except Exception:
        yield "fallback"
    else:
        yield "success"
"#;
        let path = PathBuf::from("conftest.py");
        let fixtures = extract_fixtures_from_source(source, &path).unwrap();
        let fixture = fixtures.get("fix_except").unwrap();
        assert!(
            fixture.is_yield,
            "yield inside except handler should be detected"
        );
    }

    #[test]
    fn test_conftest_map_autouse() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        std::fs::write(
            root.join("conftest.py"),
            r#"
import pytest

@pytest.fixture(autouse=True)
def auto_fix():
    pass

@pytest.fixture
def normal_fix():
    pass
"#,
        )
        .unwrap();

        let map = ConftestMap::discover(root).unwrap();
        let autouse = map.autouse_names();
        assert_eq!(autouse, vec!["auto_fix"]);
    }
}
