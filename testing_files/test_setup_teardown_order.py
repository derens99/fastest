"""Test to verify correct setup/teardown execution order"""

# Track execution order
execution_order = []


def setup_module(module):
    """Module setup - should be called first"""
    execution_order.append('setup_module')


def teardown_module(module):
    """Module teardown - should be called last"""
    execution_order.append('teardown_module')


class TestExecutionOrder:
    """Test class to verify setup/teardown order"""
    
    @classmethod
    def setup_class(cls):
        """Class setup - should be called after module setup"""
        execution_order.append('setup_class')
    
    @classmethod
    def teardown_class(cls):
        """Class teardown - should be called before module teardown"""
        execution_order.append('teardown_class')
    
    def setup_method(self, method):
        """Method setup - should be called before each test"""
        execution_order.append(f'setup_method_{method.__name__}')
    
    def teardown_method(self, method):
        """Method teardown - should be called after each test"""
        execution_order.append(f'teardown_method_{method.__name__}')
    
    def test_first(self):
        """First test method"""
        execution_order.append('test_first')
        # At this point we should have:
        # setup_module, setup_class, setup_method_test_first
        assert execution_order[0] == 'setup_module'
        assert execution_order[1] == 'setup_class'
        assert execution_order[2] == 'setup_method_test_first'
        assert execution_order[3] == 'test_first'
    
    def test_second(self):
        """Second test method"""
        execution_order.append('test_second')
        # Previous test should have completed teardown
        assert 'teardown_method_test_first' in execution_order


class TestAnotherClass:
    """Another test class to verify inter-class ordering"""
    
    @classmethod
    def setup_class(cls):
        """Setup for second class"""
        execution_order.append('setup_class_2')
        # First class should be completely done
        assert 'teardown_class' in execution_order
    
    @classmethod 
    def teardown_class(cls):
        """Teardown for second class"""
        execution_order.append('teardown_class_2')
    
    def test_in_second_class(self):
        """Test in second class"""
        execution_order.append('test_in_second_class')
        assert True


def test_module_level_after_classes():
    """Module-level test after class tests"""
    execution_order.append('test_module_level_after_classes')
    # Both classes should be torn down
    assert 'teardown_class' in execution_order
    assert 'teardown_class_2' in execution_order


def test_verify_final_order():
    """Final test to verify complete execution order"""
    execution_order.append('test_verify_final_order')
    
    # Expected order (before teardowns):
    # 1. setup_module
    # 2. setup_class
    # 3. setup_method_test_first
    # 4. test_first
    # 5. teardown_method_test_first
    # 6. setup_method_test_second
    # 7. test_second
    # 8. teardown_method_test_second
    # 9. teardown_class
    # 10. setup_class_2
    # 11. test_in_second_class
    # 12. teardown_class_2
    # 13. test_module_level_after_classes
    # 14. test_verify_final_order
    
    # Verify key ordering constraints
    assert execution_order.index('setup_module') < execution_order.index('setup_class')
    assert execution_order.index('setup_class') < execution_order.index('setup_method_test_first')
    assert execution_order.index('test_first') < execution_order.index('teardown_method_test_first')
    assert execution_order.index('teardown_class') < execution_order.index('setup_class_2')
    
    print(f"Execution order: {execution_order}")