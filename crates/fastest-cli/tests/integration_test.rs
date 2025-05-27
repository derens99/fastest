use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_version_command() {
    let mut cmd = Command::cargo_bin("fastest").unwrap();
    cmd.arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("fastest"));
}

#[test]
fn test_discover_command() {
    // Test that discover command works (it will use default paths from config)
    let mut cmd = Command::cargo_bin("fastest").unwrap();
    cmd.arg("discover")
        .assert()
        .success()
        .stdout(predicate::str::contains("Test Discovery Results"));
}

#[test]
fn test_run_simple_test() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_example.py");

    fs::write(
        &test_file,
        r#"
def test_passing():
    assert True
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastest").unwrap();
    cmd.arg(test_file.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("1 test"));
}

#[test]
fn test_run_failing_test() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_example.py");

    fs::write(
        &test_file,
        r#"
def test_failing():
    assert False, "This should fail"
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastest").unwrap();
    cmd.arg(test_file.to_str().unwrap())
        .assert()
        .failure()
        .stdout(predicate::str::contains("FAILED"));
}

#[test]
fn test_filter_by_name() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_example.py");

    fs::write(
        &test_file,
        r#"
def test_foo():
    assert True

def test_bar():
    assert True
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastest").unwrap();
    cmd.arg(test_file.to_str().unwrap())
        .arg("-k")
        .arg("foo")
        .assert()
        .success()
        .stdout(predicate::str::contains("Found 2 tests"));
}

#[test]
fn test_class_based_tests() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_class.py");

    fs::write(
        &test_file,
        r#"
class TestMath:
    def test_addition(self):
        assert 1 + 1 == 2
    
    def test_subtraction(self):
        assert 3 - 1 == 2
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastest").unwrap();
    cmd.arg(test_file.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("Found 2 tests"));
}

#[test]
fn test_async_test() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_async.py");

    fs::write(
        &test_file,
        r#"
import asyncio

async def test_async_function():
    await asyncio.sleep(0.001)
    assert True
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastest").unwrap();
    cmd.arg(test_file.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("Found 1 test"));
}
