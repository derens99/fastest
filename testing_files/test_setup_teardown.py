"""Test file to verify setup/teardown functionality"""
import pytest

# Module-level tracking
module_state = {
    'setup_module_called': False,
    'teardown_module_called': False,
    'tests_run': []
}

# Class-level tracking
class_state = {
    'setup_class_called': False,
    'teardown_class_called': False,
    'setup_method_calls': 0,
    'teardown_method_calls': 0,
    'setup_calls': 0,
    'teardown_calls': 0
}


def setup_module(module):
    """Module-level setup"""
    module_state['setup_module_called'] = True
    print(f"setup_module called for {module.__name__}")


def teardown_module(module):
    """Module-level teardown"""
    module_state['teardown_module_called'] = True
    print(f"teardown_module called for {module.__name__}")


def setup_function(function):
    """Function-level setup (for functions outside classes)"""
    print(f"setup_function called for {function.__name__}")


def teardown_function(function):
    """Function-level teardown (for functions outside classes)"""
    print(f"teardown_function called for {function.__name__}")


def test_simple_function():
    """Simple test function to verify setup_function/teardown_function"""
    module_state['tests_run'].append('test_simple_function')
    assert module_state['setup_module_called']
    assert True


def test_another_function():
    """Another test function"""
    module_state['tests_run'].append('test_another_function')
    assert True


class TestSetupTeardown:
    """Test class with various setup/teardown methods"""
    
    @classmethod
    def setup_class(cls):
        """Class-level setup"""
        class_state['setup_class_called'] = True
        print(f"setup_class called for {cls.__name__}")
    
    @classmethod
    def teardown_class(cls):
        """Class-level teardown"""
        class_state['teardown_class_called'] = True
        print(f"teardown_class called for {cls.__name__}")
    
    def setup_method(self, method):
        """Method-level setup (pytest style)"""
        class_state['setup_method_calls'] += 1
        print(f"setup_method called for {method.__name__}")
    
    def teardown_method(self, method):
        """Method-level teardown (pytest style)"""
        class_state['teardown_method_calls'] += 1
        print(f"teardown_method called for {method.__name__}")
    
    def test_class_method_one(self):
        """First test method in class"""
        module_state['tests_run'].append('test_class_method_one')
        assert class_state['setup_class_called']
        assert class_state['setup_method_calls'] > 0
    
    def test_class_method_two(self):
        """Second test method in class"""
        module_state['tests_run'].append('test_class_method_two')
        assert class_state['setup_class_called']
        assert class_state['setup_method_calls'] > 0


class TestUnittestStyle:
    """Test class with unittest-style setUp/tearDown"""
    
    def setUp(self):
        """unittest-style setup"""
        class_state['setup_calls'] += 1
        print(f"setUp called")
    
    def tearDown(self):
        """unittest-style teardown"""
        class_state['teardown_calls'] += 1
        print(f"tearDown called")
    
    def test_unittest_style_one(self):
        """First unittest-style test"""
        module_state['tests_run'].append('test_unittest_style_one')
        assert class_state['setup_calls'] > 0
    
    def test_unittest_style_two(self):
        """Second unittest-style test"""
        module_state['tests_run'].append('test_unittest_style_two')
        assert class_state['setup_calls'] > 0


class TestFailingSetup:
    """Test class where setup fails"""
    
    @classmethod
    def setup_class(cls):
        """This setup will fail"""
        raise RuntimeError("Intentional setup failure")
    
    def test_should_be_skipped(self):
        """This test should be skipped due to setup failure"""
        pytest.fail("This test should not run")


class TestAsyncSetupTeardown:
    """Test class with async tests and setup/teardown"""
    
    def setup_method(self, method):
        """Setup for async tests"""
        print(f"Setting up async test: {method.__name__}")
    
    def teardown_method(self, method):
        """Teardown for async tests"""
        print(f"Tearing down async test: {method.__name__}")
    
    @pytest.mark.asyncio
    async def test_async_with_setup(self):
        """Async test with setup/teardown"""
        module_state['tests_run'].append('test_async_with_setup')
        assert True


# Parametrized tests with setup/teardown
class TestParametrizedWithSetup:
    """Test parametrized tests with setup/teardown"""
    
    def setup_method(self, method):
        """Setup before each parametrized test"""
        print(f"Setup for parametrized test: {method.__name__}")
    
    def teardown_method(self, method):
        """Teardown after each parametrized test"""
        print(f"Teardown for parametrized test: {method.__name__}")
    
    @pytest.mark.parametrize("value", [1, 2, 3])
    def test_parametrized(self, value):
        """Parametrized test with setup/teardown"""
        module_state['tests_run'].append(f'test_parametrized[{value}]')
        assert value > 0