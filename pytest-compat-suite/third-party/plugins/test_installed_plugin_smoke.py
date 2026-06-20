"""Smoke tests for installed third-party pytest plugin packages.

This suite is intentionally small. It proves that the development environment
contains real plugin packages and that Fastest can run the subset of plugin
style behavior currently covered by its shims.
"""

import asyncio
import importlib.metadata

import pytest


PLUGIN_DISTRIBUTIONS = [
    "pytest-asyncio",
    "pytest-cov",
    "pytest-mock",
    "pytest-timeout",
    "pytest-xdist",
]


def test_expected_third_party_plugin_packages_are_installed():
    versions = {
        distribution: importlib.metadata.version(distribution)
        for distribution in PLUGIN_DISTRIBUTIONS
    }

    assert set(versions) == set(PLUGIN_DISTRIBUTIONS)
    assert all(version for version in versions.values())


def test_pytest_mock_mocker_fixture_smoke(mocker):
    target = mocker.Mock(return_value="plugin-smoke")

    assert target() == "plugin-smoke"
    target.assert_called_once_with()


@pytest.mark.asyncio
async def test_pytest_asyncio_mark_smoke():
    result = await asyncio.sleep(0, result="async-smoke")

    assert result == "async-smoke"
    assert asyncio.get_running_loop().is_running()


@pytest.mark.timeout(2)
def test_pytest_timeout_marker_smoke(request):
    marker = request.node.get_closest_marker("timeout")

    assert marker is not None
    assert [str(arg) for arg in marker.args] == ["2"]
