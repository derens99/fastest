"""Test error handling in setup/teardown methods"""
import pytest


# Track what gets called even with failures
calls_made = []


def setup_module(module):
    """Module setup that succeeds"""
    calls_made.append('setup_module')


def teardown_module(module):
    """Module teardown should still be called even if tests fail"""
    calls_made.append('teardown_module')


class TestSetupFailure:
    """Test class where setup fails"""
    
    @classmethod
    def setup_class(cls):
        """This setup will fail"""
        calls_made.append('setup_class_failing')
        raise RuntimeError("Intentional setup_class failure")
    
    @classmethod
    def teardown_class(cls):
        """This should not be called since setup failed"""
        calls_made.append('teardown_class_failing')
    
    def test_skipped_due_to_setup_failure(self):
        """This test should not run due to setup_class failure"""
        calls_made.append('test_should_not_run')
        pytest.fail("This test should not execute")


class TestTeardownFailure:
    """Test class where teardown fails"""
    
    @classmethod
    def setup_class(cls):
        """Setup succeeds"""
        calls_made.append('setup_class_teardown_fail')
    
    @classmethod
    def teardown_class(cls):
        """This teardown will fail"""
        calls_made.append('teardown_class_will_fail')
        raise RuntimeError("Intentional teardown_class failure")
    
    def test_runs_despite_teardown_failure(self):
        """This test should run normally"""
        calls_made.append('test_runs_normally')
        assert True


class TestMethodSetupFailure:
    """Test where setup_method fails"""
    
    def setup_method(self, method):
        """This setup will fail for specific test"""
        calls_made.append(f'setup_method_{method.__name__}')
        if method.__name__ == 'test_with_failing_setup':
            raise RuntimeError("Intentional setup_method failure")
    
    def teardown_method(self, method):
        """Teardown should be called for successful tests"""
        calls_made.append(f'teardown_method_{method.__name__}')
    
    def test_normal(self):
        """This test should run normally"""
        calls_made.append('test_normal')
        assert True
    
    def test_with_failing_setup(self):
        """This test should not run due to setup_method failure"""
        calls_made.append('test_with_failing_setup_body')
        pytest.fail("Should not reach test body")
    
    def test_after_failure(self):
        """This test should still run after previous test's setup failure"""
        calls_made.append('test_after_failure')
        assert True


class TestExceptionInTest:
    """Test where the test itself fails"""
    
    def setup_method(self, method):
        """Setup that succeeds"""
        calls_made.append(f'setup_for_{method.__name__}')
    
    def teardown_method(self, method):
        """Teardown should still be called even if test fails"""
        calls_made.append(f'teardown_for_{method.__name__}')
    
    def test_that_fails(self):
        """Test that raises an exception"""
        calls_made.append('test_that_fails')
        raise ValueError("Test failure")
    
    def test_after_failed_test(self):
        """Verify previous test's teardown was called"""
        calls_made.append('test_after_failed_test')
        assert f'teardown_for_test_that_fails' in calls_made


def test_verify_error_handling():
    """Verify error handling worked correctly"""
    calls_made.append('test_verify_error_handling')
    
    print("Calls made despite errors:")
    for call in calls_made:
        print(f"  - {call}")
    
    # Verify key behaviors:
    # 1. Module setup was called
    assert 'setup_module' in calls_made
    
    # 2. Tests didn't run when class setup failed
    assert 'test_should_not_run' not in calls_made
    
    # 3. Teardown wasn't called when setup failed
    assert 'teardown_class_failing' not in calls_made
    
    # 4. Tests ran even though teardown would fail
    assert 'test_runs_normally' in calls_made
    
    # 5. Test body didn't run when method setup failed
    assert 'test_with_failing_setup_body' not in calls_made
    
    # 6. Other tests still ran after method setup failure
    assert 'test_after_failure' in calls_made
    
    # 7. Teardown was called even when test failed
    assert 'test_that_fails' in calls_made
    assert 'teardown_for_test_that_fails' in calls_made