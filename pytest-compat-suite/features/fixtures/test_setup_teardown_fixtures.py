"""Test setup/teardown interaction with fixtures"""
import pytest


# Track setup/teardown and fixture calls
call_order = []


def setup_module(module):
    """Module setup should run before any fixtures"""
    call_order.append('setup_module')


def teardown_module(module):
    """Module teardown should run after all fixtures are torn down"""
    call_order.append('teardown_module')


@pytest.fixture(scope="module")
def module_fixture():
    """Module-scoped fixture"""
    call_order.append('module_fixture_setup')
    yield "module_data"
    call_order.append('module_fixture_teardown')


@pytest.fixture(scope="class")
def class_fixture():
    """Class-scoped fixture"""
    call_order.append('class_fixture_setup')
    yield "class_data"
    call_order.append('class_fixture_teardown')


@pytest.fixture
def function_fixture():
    """Function-scoped fixture"""
    call_order.append('function_fixture_setup')
    yield "function_data"
    call_order.append('function_fixture_teardown')


class TestSetupTeardownWithFixtures:
    """Test class that uses both setup/teardown and fixtures"""
    
    @classmethod
    def setup_class(cls):
        """Class setup should run after module setup but can be before/after module fixtures"""
        call_order.append('setup_class')
    
    @classmethod
    def teardown_class(cls):
        """Class teardown"""
        call_order.append('teardown_class')
    
    def setup_method(self, method):
        """Method setup should run after class-level setup"""
        call_order.append(f'setup_method_{method.__name__}')
    
    def teardown_method(self, method):
        """Method teardown should run after test but can be before/after function fixture teardown"""
        call_order.append(f'teardown_method_{method.__name__}')
    
    def test_with_all_fixtures(self, module_fixture, class_fixture, function_fixture):
        """Test using all fixture scopes"""
        call_order.append('test_with_all_fixtures')
        
        # Verify setup order
        assert call_order.index('setup_module') < call_order.index('setup_class')
        assert call_order.index('setup_class') <= call_order.index('class_fixture_setup')
        assert call_order.index('setup_method_test_with_all_fixtures') < call_order.index('test_with_all_fixtures')
        
        # Verify fixtures are available
        assert module_fixture == "module_data"
        assert class_fixture == "class_data"
        assert function_fixture == "function_data"
    
    def test_another_with_fixture(self, function_fixture):
        """Another test with fixture"""
        call_order.append('test_another_with_fixture')
        
        # Function fixture should be set up fresh for this test
        # Find the last occurrence of function_fixture_setup
        setup_indices = [i for i, x in enumerate(call_order) if x == 'function_fixture_setup']
        assert len(setup_indices) >= 2  # Should be called for each test
        
        assert function_fixture == "function_data"


def test_module_function_with_fixture(function_fixture):
    """Module-level test function with fixture"""
    call_order.append('test_module_function_with_fixture')
    assert function_fixture == "function_data"
    
    # Class should be completely torn down by now
    assert 'teardown_class' in call_order


def test_final_order_verification():
    """Verify the complete execution order"""
    call_order.append('test_final_order_verification')
    
    # Print for debugging
    print("Complete call order:")
    for i, call in enumerate(call_order):
        print(f"{i}: {call}")
    
    # Key invariants:
    # 1. Module setup is first
    assert call_order[0] == 'setup_module'
    
    # 2. setup_method comes before the test
    for i, call in enumerate(call_order):
        if call.startswith('test_') and call != 'test_final_order_verification':
            # Find the corresponding setup_method
            setup_call = f'setup_method_{call}'
            if setup_call in call_order:
                assert call_order.index(setup_call) < i
    
    # 3. Fixture setup comes before its use in test
    test_indices = [i for i, call in enumerate(call_order) if call == 'test_with_all_fixtures']
    if test_indices:
        test_index = test_indices[0]
        assert call_order.index('module_fixture_setup') < test_index
        assert call_order.index('class_fixture_setup') < test_index