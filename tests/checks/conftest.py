"""Conftest fixtures for the checks test suite."""

import pytest


@pytest.fixture
def sample_data():
    return [1, 2, 3, 4, 5]


@pytest.fixture
def greeting():
    return "Hello, Fastest!"


@pytest.fixture
def app_config():
    return {
        "debug": True,
        "name": "test_app",
        "version": "1.0",
    }
