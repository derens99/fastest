use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

#[test]
fn test_version() {
    Command::cargo_bin("fastest")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("fastest"));
}

#[test]
fn test_help() {
    Command::cargo_bin("fastest")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Blazing-fast Python test runner"));
}

#[test]
fn test_discover_basic() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("test_example.py"),
        r#"
def test_one():
    assert True

def test_two():
    assert True
"#,
    )
    .unwrap();

    Command::cargo_bin("fastest")
        .unwrap()
        .arg("discover")
        .arg(dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("test_one"))
        .stdout(predicate::str::contains("test_two"));
}

#[test]
fn test_discover_json() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("test_example.py"),
        r#"
def test_hello():
    pass
"#,
    )
    .unwrap();

    Command::cargo_bin("fastest")
        .unwrap()
        .arg("discover")
        .arg("--output")
        .arg("json")
        .arg(dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("test_hello"))
        .stdout(predicate::str::contains("function_name"));
}

#[test]
fn test_discover_count() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("test_example.py"),
        r#"
def test_a():
    pass

def test_b():
    pass
"#,
    )
    .unwrap();

    Command::cargo_bin("fastest")
        .unwrap()
        .arg("discover")
        .arg("--output")
        .arg("count")
        .arg(dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("2 tests discovered"));
}

#[test]
fn test_no_tests_found() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("helper.py"), "def helper(): pass").unwrap();

    Command::cargo_bin("fastest")
        .unwrap()
        .arg(dir.path().to_str().unwrap())
        .assert()
        .success();
}

#[test]
fn test_discover_class_based_tests() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("test_classes.py"),
        r#"
class TestMath:
    def test_add(self):
        assert 1 + 1 == 2

    def test_sub(self):
        assert 2 - 1 == 1
"#,
    )
    .unwrap();

    Command::cargo_bin("fastest")
        .unwrap()
        .arg("discover")
        .arg(dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("test_add"))
        .stdout(predicate::str::contains("test_sub"));
}

#[test]
fn test_discover_subcommand_help() {
    Command::cargo_bin("fastest")
        .unwrap()
        .arg("discover")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("List discovered tests"));
}
