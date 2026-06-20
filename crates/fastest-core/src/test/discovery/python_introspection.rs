//! Python-based test discovery using PyO3 introspection
//!
//! This module uses Python's native introspection capabilities to discover tests,
//! which ensures perfect compatibility with all Python features including Unicode identifiers.

use crate::error::Result;
use crate::test::discovery::TestItem;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Discover tests in a Python file using Python introspection
pub fn discover_tests_python(file_path: &Path) -> Result<Vec<TestItem>> {
    Python::with_gil(|py| {
        discover_tests_in_file_py(py, file_path)
            .map_err(|e| crate::error::Error::Discovery(format!("Python discovery failed: {}", e)))
    })
}

fn discover_tests_in_file_py(py: Python<'_>, file_path: &Path) -> PyResult<Vec<TestItem>> {
    // Create the discovery code that will run in Python
    let discovery_code = r#"
import ast
import importlib.util
import sys
import os
from pathlib import Path

def discover_tests_in_file(file_path):
    """Discover all test functions and methods in a Python file."""
    tests = []

    # Read the file content
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    # Parse the AST
    try:
        tree = ast.parse(content, filename=str(file_path))
    except SyntaxError as e:
        return {'error': f'Syntax error in {file_path}: {e}'}

    # Extract module name from path
    module_name = Path(file_path).stem

    def decorator_name(decorator):
        """Return a compact decorator representation."""
        try:
            return f"@{ast.unparse(decorator)}"
        except Exception:
            if hasattr(decorator, 'id'):
                return f"@{decorator.id}"
            if hasattr(decorator, 'attr'):
                return f"@{decorator.attr}"
            return "@decorator"

    def extract_pytestmark(value):
        """Extract module-level pytestmark entries as decorator strings."""
        if isinstance(value, (ast.List, ast.Tuple)):
            values = value.elts
        else:
            values = [value]
        return [decorator_name(item) for item in values]

    module_decorators = []
    for node in tree.body:
        if isinstance(node, ast.Assign):
            for target in node.targets:
                if isinstance(target, ast.Name) and target.id == "pytestmark":
                    module_decorators.extend(extract_pytestmark(node.value))
        elif isinstance(node, ast.AnnAssign):
            target = node.target
            if isinstance(target, ast.Name) and target.id == "pytestmark" and node.value:
                module_decorators.extend(extract_pytestmark(node.value))

    def test_info(node, class_name=None, inherited_decorators=None):
        is_async = isinstance(node, ast.AsyncFunctionDef)
        decorators = list(module_decorators)
        decorators.extend(inherited_decorators or [])
        decorators.extend(decorator_name(decorator) for decorator in node.decorator_list)
        params = [arg.arg for arg in node.args.args if arg.arg != 'self']
        return {
            'name': node.name,
            'line_number': node.lineno,
            'is_async': is_async,
            'decorators': decorators,
            'parameters': params,
            'class_name': class_name
        }

    # Walk top-level statements only so class methods are not double-counted.
    for node in tree.body:
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            # Check if it's a test function
            if node.name.startswith('test'):
                tests.append(test_info(node))

        elif isinstance(node, ast.ClassDef):
            # Check if it's a test class
            if node.name.startswith('Test'):
                class_decorators = [
                    decorator_name(decorator) for decorator in node.decorator_list
                ]
                # Find test methods in the class
                for item in node.body:
                    if isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef)):
                        if item.name.startswith('test'):
                            tests.append(test_info(item, node.name, class_decorators))

    return {'tests': tests, 'module_name': module_name}
"#;

    // Execute the discovery code - first define the function
    let namespace = PyDict::new(py);

    // Convert discovery_code to CStr (null-terminated string)
    let code_cstr = std::ffi::CString::new(discovery_code)
        .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid code string"))?;

    // Run the code to define the function
    py.run(code_cstr.as_c_str(), Some(&namespace), Some(&namespace))?;

    // Now call the function from the shared namespace. Using one dictionary keeps
    // imports such as `ast` visible to the function when it executes.
    let discover_func = namespace
        .get_item("discover_tests_in_file")
        .ok()
        .flatten()
        .ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "discover_tests_in_file function not found",
            )
        })?;

    let result = discover_func.call1((file_path.to_str().unwrap(),))?;

    // Extract the result dictionary
    let result_dict = result.downcast::<PyDict>()?;

    // Check for errors
    if let Some(error) = result_dict.get_item("error")? {
        if let Ok(error_str) = error.extract::<String>() {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error_str));
        }
    }

    // Extract tests
    let tests_list = result_dict.get_item("tests")?.unwrap();
    let _module_name = result_dict
        .get_item("module_name")?
        .unwrap()
        .extract::<String>()
        .unwrap_or_else(|_| file_path.file_stem().unwrap().to_str().unwrap().to_string());

    let tests = tests_list.downcast::<PyList>()?;

    let mut test_items = Vec::new();

    for test in tests {
        let test_dict = test.downcast::<PyDict>()?;

        let name: String = test_dict.get_item("name")?.unwrap().extract()?;
        let line_number: usize = test_dict.get_item("line_number")?.unwrap().extract()?;
        let is_async: bool = test_dict.get_item("is_async")?.unwrap().extract()?;
        let class_name: Option<String> = test_dict
            .get_item("class_name")?
            .unwrap()
            .extract()
            .ok()
            .filter(|s: &String| s != "None");
        let decorators: Vec<String> = test_dict.get_item("decorators")?.unwrap().extract()?;
        let parameters: Vec<String> = test_dict.get_item("parameters")?.unwrap().extract()?;

        // Create test ID
        let test_id = if let Some(ref class) = class_name {
            format!("{}::{}::{}", file_path.display(), class, name)
        } else {
            format!("{}::{}", file_path.display(), name)
        };

        let test_item = TestItem {
            id: test_id,
            path: file_path.to_path_buf(),
            name: name.clone(),
            function_name: name, // Original Unicode name preserved
            line_number: Some(line_number as u32),
            is_async,
            class_name,
            decorators: decorators.into_iter().collect(),
            fixture_deps: parameters.into_iter().collect(),
            is_xfail: false,
            indirect_params: HashMap::new(),
        };

        test_items.push(test_item);
    }

    Ok(test_items)
}

/// Discover tests in multiple files
pub fn discover_tests_in_files(file_paths: &[PathBuf]) -> Result<Vec<TestItem>> {
    let mut all_tests = Vec::new();

    for file_path in file_paths {
        match discover_tests_python(file_path) {
            Ok(tests) => all_tests.extend(tests),
            Err(e) => eprintln!(
                "Warning: Failed to discover tests in {:?}: {}",
                file_path, e
            ),
        }
    }

    Ok(all_tests)
}
