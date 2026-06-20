use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::process::{Command as StdCommand, Stdio};
use std::thread;
use std::time::{Duration, Instant};
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
        .stdout(predicate::str::contains("Running 1 tests"));
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
        .stdout(predicate::str::contains("Running 2 tests"));
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
        .stdout(predicate::str::contains("Running 1 tests"));
}

#[test]
fn test_same_stem_files_resolve_by_path() {
    let temp_dir = TempDir::new().unwrap();
    let first_dir = temp_dir.path().join("first");
    let second_dir = temp_dir.path().join("second");
    fs::create_dir_all(&first_dir).unwrap();
    fs::create_dir_all(&second_dir).unwrap();

    fs::write(
        first_dir.join("test_same.py"),
        r#"
def test_first_only():
    assert True
"#,
    )
    .unwrap();

    fs::write(
        second_dir.join("test_same.py"),
        r#"
def test_second_only():
    assert True
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastest").unwrap();
    cmd.arg(temp_dir.path().to_str().unwrap())
        .arg("--no-cache")
        .assert()
        .success()
        .stdout(predicate::str::contains("Running 2 tests"));
}

#[test]
fn test_pytest_import_is_available_during_execution() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_pytest_import.py");

    fs::write(
        &test_file,
        r#"
import pytest

def test_raises_context_manager():
    with pytest.raises(ValueError):
        raise ValueError("expected")
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastest").unwrap();
    cmd.arg(test_file.to_str().unwrap())
        .arg("--no-cache")
        .assert()
        .success()
        .stdout(predicate::str::contains("Running 1 tests"));
}

#[test]
fn test_pytest_raises_supports_match_keyword() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_pytest_raises_match.py");

    fs::write(
        &test_file,
        r#"
import pytest

def test_raises_match_keyword():
    with pytest.raises(ValueError, match="invalid.*value"):
        raise ValueError("invalid value provided")
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastest").unwrap();
    cmd.arg(test_file.to_str().unwrap())
        .arg("--no-cache")
        .assert()
        .success()
        .stdout(predicate::str::contains("1 passed"));
}

#[test]
fn test_conftest_autouse_fixture_can_access_request_instance() {
    let temp_dir = TempDir::new().unwrap();
    let conftest_file = temp_dir.path().join("conftest.py");
    let test_file = temp_dir.path().join("test_conftest_autouse.py");

    fs::write(
        &conftest_file,
        r#"
import pytest

@pytest.fixture(autouse=True)
def mark_instance(request):
    if request.instance is not None:
        request.instance._autouse_applied = True
    yield
"#,
    )
    .unwrap();

    fs::write(
        &test_file,
        r#"
class TestAutouseRequest:
    def test_autouse_marks_instance(self):
        assert self._autouse_applied is True
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastest").unwrap();
    cmd.arg(test_file.to_str().unwrap())
        .arg("--no-cache")
        .assert()
        .success()
        .stdout(predicate::str::contains("1 passed"));
}

#[test]
fn test_unittest_mock_call_import_does_not_hang_fixture_scan() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_mock_call_import.py");

    fs::write(
        &test_file,
        r#"
import pytest
from unittest.mock import Mock, MagicMock, patch, call

class TestMockerImport:
    def test_mocker_mock(self, mocker):
        mock = mocker.Mock(return_value=123)
        assert mock() == 123
        mock.assert_called_once()
"#,
    )
    .unwrap();

    let mut child = StdCommand::new(assert_cmd::cargo::cargo_bin("fastest"))
        .arg(test_file.to_str().unwrap())
        .arg("--no-cache")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if child.try_wait().unwrap().is_some() {
            let output = child.wait_with_output().unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                output.status.success(),
                "fastest failed\nstdout:\n{}\nstderr:\n{}",
                stdout,
                stderr
            );
            assert!(stdout.contains("1 passed"), "stdout:\n{}", stdout);
            return;
        }

        if Instant::now() >= deadline {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!(
                "fastest timed out while scanning unittest.mock.call import\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        thread::sleep(Duration::from_millis(25));
    }
}

#[test]
fn test_pytest_hook_decorators_are_available() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_hook_decorators.py");

    fs::write(
        &test_file,
        r#"
import pytest

class Plugin:
    @pytest.hookimpl(tryfirst=True)
    def pytest_runtest_setup(self, item):
        return None

    @pytest.hookspec(firstresult=True)
    def pytest_custom_hook(self):
        return None

def test_hook_decorators_are_noop_compatible():
    assert Plugin().pytest_runtest_setup(None) is None
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastest").unwrap();
    cmd.arg(test_file.to_str().unwrap())
        .arg("--no-cache")
        .assert()
        .success()
        .stdout(predicate::str::contains("1 passed"));
}

#[test]
fn test_setup_method_without_method_argument_is_supported() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_setup_method.py");

    fs::write(
        &test_file,
        r#"
class TestSetupMethod:
    def setup_method(self):
        self.ready = True

    def test_setup_ran(self):
        assert self.ready is True
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastest").unwrap();
    cmd.arg(test_file.to_str().unwrap())
        .arg("--no-cache")
        .assert()
        .success()
        .stdout(predicate::str::contains("1 passed"));
}

#[test]
fn test_request_node_marker_lookup_and_config_cache_helpers() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_request_helpers.py");

    fs::write(
        &test_file,
        r#"
import pytest

@pytest.mark.slow
def test_request_helpers(request, cache, tmpdir_factory):
    assert request.node.get_closest_marker("slow").name == "slow"
    assert request.config.workerinput["workerid"] == "master"
    cache.set("answer", 42)
    assert cache.get("answer", None) == 42
    tmpdir = tmpdir_factory.mktemp("data")
    assert tmpdir.exists()
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastest").unwrap();
    cmd.arg(test_file.to_str().unwrap())
        .arg("--no-cache")
        .assert()
        .success()
        .stdout(predicate::str::contains("1 passed"));
}

#[test]
fn test_async_event_loop_fixture_runs_test_on_same_loop() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_event_loop_fixture.py");

    fs::write(
        &test_file,
        r#"
import asyncio

async def test_event_loop_fixture(event_loop):
    assert isinstance(event_loop, asyncio.AbstractEventLoop)
    assert event_loop.is_running()
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastest").unwrap();
    cmd.arg(test_file.to_str().unwrap())
        .arg("--no-cache")
        .assert()
        .success()
        .stdout(predicate::str::contains("1 passed"));
}

#[test]
fn test_mocker_extended_helpers() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_mocker_helpers.py");

    fs::write(
        &test_file,
        r#"
class Target:
    def method(self):
        return "original"

    @property
    def value(self):
        return "original"

def test_mocker_helpers(mocker):
    target = Target()
    mocker.patch.object(target, "method", return_value="patched")
    assert target.method() == "patched"

    prop = mocker.PropertyMock(return_value="prop")
    mocker.patch.object(Target, "value", prop)
    assert target.value == "prop"

    stub = mocker.stub(name="stub")
    stub.work.return_value = 7
    assert stub.work() == 7

    opened = mocker.mock_open(read_data="content")
    mocker.patch("builtins.open", opened)
    with open("/tmp/example.txt") as handle:
        assert handle.read() == "content"

    mock = mocker.Mock()
    mock()
    mocker.resetall()
    assert not mock.called
    mocker.stopall()
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastest").unwrap();
    cmd.arg(test_file.to_str().unwrap())
        .arg("--no-cache")
        .assert()
        .success()
        .stdout(predicate::str::contains("1 passed"));
}

#[test]
fn test_pytest_param_is_available_during_execution() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_pytest_param.py");

    fs::write(
        &test_file,
        r#"
import pytest

@pytest.mark.parametrize("value,expected", [
    pytest.param(2, 4, id="double"),
])
def test_double(value, expected):
    assert value * 2 == expected
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastest").unwrap();
    cmd.arg(test_file.to_str().unwrap())
        .arg("--no-cache")
        .assert()
        .success()
        .stdout(predicate::str::contains("1 passed"));
}

#[test]
fn test_class_teardown_runs_before_following_module_test() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_lifecycle.py");

    fs::write(
        &test_file,
        r#"
events = []

class TestLifecycle:
    @classmethod
    def setup_class(cls):
        events.append("setup_class")

    @classmethod
    def teardown_class(cls):
        events.append("teardown_class")

    def test_one(self):
        events.append("test_one")

    def test_two(self):
        events.append("test_two")

def test_after_class():
    assert "teardown_class" in events
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastest").unwrap();
    cmd.arg(test_file.to_str().unwrap())
        .arg("--no-cache")
        .assert()
        .success()
        .stdout(predicate::str::contains("Running 3 tests"));
}

#[test]
fn test_xfailed_setup_class_does_not_abort_following_tests() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_xfailed_setup.py");

    fs::write(
        &test_file,
        r#"
import pytest

@pytest.mark.xfail(reason="intentional setup failure")
class TestBrokenSetup:
    @classmethod
    def setup_class(cls):
        raise RuntimeError("setup exploded")

    def test_expected_failure(self):
        assert True

class TestAfterBrokenSetup:
    def test_still_runs(self):
        assert True
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastest").unwrap();
    cmd.arg(test_file.to_str().unwrap())
        .arg("--no-cache")
        .assert()
        .success()
        .stdout(predicate::str::contains("1 passed"))
        .stdout(predicate::str::contains("1 xfailed"));
}

#[test]
fn test_non_strict_xpass_does_not_fail_run() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_xpass.py");

    fs::write(
        &test_file,
        r#"
import pytest

@pytest.mark.xfail(reason="non-strict expected failure")
def test_unexpected_pass():
    assert True
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastest").unwrap();
    cmd.arg(test_file.to_str().unwrap())
        .arg("--no-cache")
        .assert()
        .success()
        .stdout(predicate::str::contains("1 xpassed"));
}

#[test]
fn test_skipif_platform_expression_skips_test() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_skipif_platform.py");

    fs::write(
        &test_file,
        r#"
import platform
import pytest

@pytest.mark.skipif(platform.system() == platform.system(), reason="same platform")
def test_platform_skip():
    assert False
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastest").unwrap();
    cmd.arg(test_file.to_str().unwrap())
        .arg("--no-cache")
        .assert()
        .success()
        .stdout(predicate::str::contains("1 skipped"));
}

#[test]
fn test_strict_xpass_fails_run() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_strict_xpass.py");

    fs::write(
        &test_file,
        r#"
import pytest

@pytest.mark.xfail(strict=True)
def test_unexpected_pass_is_failure():
    assert True
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastest").unwrap();
    cmd.arg(test_file.to_str().unwrap())
        .arg("--no-cache")
        .assert()
        .failure()
        .stdout(predicate::str::contains("FAILED"));
}

#[test]
fn test_xfail_raises_wrong_exception_fails_run() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_xfail_raises.py");

    fs::write(
        &test_file,
        r#"
import pytest

@pytest.mark.xfail(raises=TypeError)
def test_wrong_exception_type():
    raise ValueError("wrong type")
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastest").unwrap();
    cmd.arg(test_file.to_str().unwrap())
        .arg("--no-cache")
        .assert()
        .failure()
        .stdout(predicate::str::contains("FAILED"));
}

#[test]
fn test_parametrize_safe_static_expressions() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_static_parametrize.py");

    fs::write(
        &test_file,
        r#"
import math
import pytest

@pytest.mark.parametrize("number", range(3))
def test_range_values(number):
    assert number in {0, 1, 2}

@pytest.mark.parametrize("special", [float("inf"), float("-inf"), float("nan")])
def test_special_float_values(special):
    assert isinstance(special, float)
    assert math.isinf(special) or math.isnan(special)

@pytest.mark.parametrize("empty", [set()])
def test_set_constructor_value(empty):
    assert isinstance(empty, set)
    assert len(empty) == 0

@pytest.mark.parametrize("text", ["a" * 100])
def test_repeated_string_value(text):
    assert text == "a" * 100
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastest").unwrap();
    cmd.arg(test_file.to_str().unwrap())
        .arg("--no-cache")
        .assert()
        .success()
        .stdout(predicate::str::contains("8 passed"));
}
