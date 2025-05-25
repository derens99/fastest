use pyo3::prelude::*;

#[pyfunction]
fn run_tests() -> PyResult<()> {
    println!("fastest: running tests (stub)");
    Ok(())
}

#[pymodule]
fn fastest(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(run_tests, m)?)?;
    Ok(())
}
