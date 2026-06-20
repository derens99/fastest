"""Shared fixtures for the comprehensive compatibility suite."""

import pytest


def pytest_configure(config):
    config.addinivalue_line("markers", "custom_mark: custom marker used by conftest tests")
    config.addinivalue_line(
        "markers", "custom_plugin_marker: custom marker used by plugin tests"
    )
    config.addinivalue_line("markers", "integration: integration-style compatibility test")
    config.addinivalue_line("markers", "slow: slow compatibility test")
    config.addinivalue_line("markers", "timeout(seconds): timeout compatibility marker")


@pytest.fixture
def conftest_fixture():
    return "conftest_value"


@pytest.fixture
def parent_fixture():
    return "parent_value"


@pytest.fixture
def another_fixture():
    return 42


@pytest.fixture(scope="module")
def module_data():
    return {"shared": "data"}


@pytest.fixture(scope="session")
def session_data():
    return {"session": "info"}


@pytest.fixture(params=[1, 2, 3])
def param_fixture(request):
    return request.param


@pytest.fixture
def plugin_fixture():
    return "plugin_value"


@pytest.fixture(autouse=True)
def mark_autouse_applied(request):
    instance = getattr(request, "instance", None)
    if instance is not None:
        instance._autouse_applied = True
    yield
