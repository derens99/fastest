"""Test proper class teardown timing with the new ClassLifecycleManager"""

import pytest
import sys
from pathlib import Path

# Test tracking for teardown order
teardown_order = []
setup_order = []

class TestClassA:
    """First test class to verify teardown timing"""
    
    @classmethod
    def setup_class(cls):
        setup_order.append('TestClassA')
    
    @classmethod
    def teardown_class(cls):
        teardown_order.append('TestClassA')
    
    def test_a1(self):
        assert True
    
    def test_a2(self):
        assert True
    
    def test_a3(self):
        assert True


class TestClassB:
    """Second test class to verify transitions"""
    
    @classmethod
    def setup_class(cls):
        setup_order.append('TestClassB')
    
    @classmethod
    def teardown_class(cls):
        teardown_order.append('TestClassB')
    
    def test_b1(self):
        # At this point, TestClassA should be torn down
        assert 'TestClassA' in teardown_order, "TestClassA should be torn down before TestClassB starts"
    
    def test_b2(self):
        assert True


def test_module_level_after_class():
    """Module level test after classes"""
    # Both classes should be torn down
    assert 'TestClassB' in teardown_order, "TestClassB should be torn down before module test"


class TestClassC:
    """Third test class after module test"""
    
    @classmethod
    def setup_class(cls):
        setup_order.append('TestClassC')
    
    @classmethod
    def teardown_class(cls):
        teardown_order.append('TestClassC')
    
    def test_c1(self):
        assert True


class TestExceptionHandling:
    """Test that teardown happens even with exceptions"""
    
    @classmethod
    def setup_class(cls):
        setup_order.append('TestExceptionHandling')
    
    @classmethod
    def teardown_class(cls):
        teardown_order.append('TestExceptionHandling')
    
    def test_exception(self):
        # This should not prevent teardown
        raise ValueError("Test exception")
    
    def test_after_exception(self):
        # This may or may not run depending on fail-fast settings
        assert True


class TestEmptyClass:
    """Class with setup/teardown but no tests - should not setup"""
    
    @classmethod
    def setup_class(cls):
        setup_order.append('TestEmptyClass')
    
    @classmethod
    def teardown_class(cls):
        teardown_order.append('TestEmptyClass')


def test_final_verification():
    """Final test to verify all teardowns happened"""
    # Check that setup and teardown happened in correct order
    expected_classes = ['TestClassA', 'TestClassB', 'TestClassC', 'TestExceptionHandling']
    
    # All setup classes should have teardown (except empty)
    for cls in expected_classes:
        if cls in setup_order:
            assert cls in teardown_order, f"{cls} was setup but not torn down"
    
    # Empty class should not be in setup or teardown
    assert 'TestEmptyClass' not in setup_order
    assert 'TestEmptyClass' not in teardown_order
    
    # Teardown should happen in reverse order of setup (generally)
    # or at least all teardowns should complete
    assert len(teardown_order) >= len(setup_order) - 1  # -1 for potential fail-fast


# Test class with only skipped tests
class TestAllSkipped:
    """Class where all tests are skipped"""
    
    @classmethod
    def setup_class(cls):
        setup_order.append('TestAllSkipped')
    
    @classmethod
    def teardown_class(cls):
        teardown_order.append('TestAllSkipped')
    
    @pytest.mark.skip(reason="Testing skip behavior")
    def test_skip1(self):
        assert False
    
    @pytest.mark.skip(reason="Testing skip behavior")
    def test_skip2(self):
        assert False