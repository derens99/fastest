"""Check 9: Conftest fixture support."""

import pytest


# These tests use fixtures defined in conftest.py
def test_sample_data(sample_data):
    assert sample_data == [1, 2, 3, 4, 5]


def test_greeting(greeting):
    assert greeting == "Hello, Fastest!"


def test_config_fixture(app_config):
    assert app_config["debug"] is True
    assert app_config["name"] == "test_app"
