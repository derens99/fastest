use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use fastest_core::{discover_tests as core_discover, run_test as core_run_test, TestItem, BatchExecutor, ParallelExecutor};

#[pyclass]
struct PyTestItem {
    #[pyo3(get)]
    id: String,
    #[pyo3(get)]
    path: String,
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    function_name: String,
    #[pyo3(get)]
    line_number: usize,
    #[pyo3(get)]
    is_async: bool,
    #[pyo3(get)]
    class_name: Option<String>,
}

#[pyclass]
struct PyTestResult {
    #[pyo3(get)]
    test_id: String,
    #[pyo3(get)]
    passed: bool,
    #[pyo3(get)]
    duration: f64,
    #[pyo3(get)]
    output: String,
    #[pyo3(get)]
    error: Option<String>,
    #[pyo3(get)]
    stdout: String,
    #[pyo3(get)]
    stderr: String,
}

#[pyfunction]
fn discover_tests(path: String) -> PyResult<Vec<PyTestItem>> {
    let tests = core_discover(std::path::Path::new(&path))
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    
    Ok(tests.into_iter().map(|t| PyTestItem {
        id: t.id,
        path: t.path.display().to_string(),
        name: t.name,
        function_name: t.function_name,
        line_number: t.line_number,
        is_async: t.is_async,
        class_name: t.class_name,
    }).collect())
}

#[pyfunction]
fn run_test(test_item: &PyTestItem) -> PyResult<PyTestResult> {
    let item = TestItem {
        id: test_item.id.clone(),
        path: std::path::PathBuf::from(&test_item.path),
        name: test_item.name.clone(),
        function_name: test_item.function_name.clone(),
        line_number: test_item.line_number,
        is_async: test_item.is_async,
        class_name: test_item.class_name.clone(),
    };
    
    let result = core_run_test(&item)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    
    Ok(PyTestResult {
        test_id: result.test_id,
        passed: result.passed,
        duration: result.duration.as_secs_f64(),
        output: result.output,
        error: result.error,
        stdout: result.stdout,
        stderr: result.stderr,
    })
}

#[pyfunction]
fn run_tests_batch(test_items: Vec<PyRef<PyTestItem>>) -> PyResult<Vec<PyTestResult>> {
    // Convert PyTestItems to TestItems
    let items: Vec<TestItem> = test_items.iter().map(|test_item| TestItem {
        id: test_item.id.clone(),
        path: std::path::PathBuf::from(&test_item.path),
        name: test_item.name.clone(),
        function_name: test_item.function_name.clone(),
        line_number: test_item.line_number,
        is_async: test_item.is_async,
        class_name: test_item.class_name.clone(),
    }).collect();
    
    // Use the batch executor for much better performance
    let executor = BatchExecutor::new();
    let results = executor.execute_tests(items);
    
    Ok(results.into_iter().map(|result| PyTestResult {
        test_id: result.test_id,
        passed: result.passed,
        duration: result.duration.as_secs_f64(),
        output: result.output,
        error: result.error,
        stdout: result.stdout,
        stderr: result.stderr,
    }).collect())
}

#[pyfunction]
#[pyo3(signature = (test_items, num_workers=None))]
fn run_tests_parallel(
    py: Python,
    test_items: Vec<PyRef<PyTestItem>>,
    num_workers: Option<usize>
) -> PyResult<Vec<PyTestResult>> {
    // Convert PyTestItems to TestItems before releasing the GIL
    let items: Vec<TestItem> = test_items.iter().map(|test_item| TestItem {
        id: test_item.id.clone(),
        path: std::path::PathBuf::from(&test_item.path),
        name: test_item.name.clone(),
        function_name: test_item.function_name.clone(),
        line_number: test_item.line_number,
        is_async: test_item.is_async,
        class_name: test_item.class_name.clone(),
    }).collect();
    
    // Now we can release the GIL
    py.allow_threads(|| {
        // Use the parallel executor
        let executor = ParallelExecutor::new(num_workers, false);
        let results = executor.execute(items)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        
        Ok(results.into_iter().map(|result| PyTestResult {
            test_id: result.test_id,
            passed: result.passed,
            duration: result.duration.as_secs_f64(),
            output: result.output,
            error: result.error,
            stdout: result.stdout,
            stderr: result.stderr,
        }).collect())
    })
}

#[pymodule]
fn fastest(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(discover_tests, m)?)?;
    m.add_function(wrap_pyfunction!(run_test, m)?)?;
    m.add_function(wrap_pyfunction!(run_tests_batch, m)?)?;
    m.add_function(wrap_pyfunction!(run_tests_parallel, m)?)?;
    m.add_class::<PyTestItem>()?;
    m.add_class::<PyTestResult>()?;
    m.add("__version__", "0.2.0")?;
    Ok(())
}