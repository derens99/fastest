"""Test to verify that teardown_class is called when transitioning between classes"""

# Track setup/teardown calls
setup_teardown_log = []

class TestClassA:
    @classmethod
    def setup_class(cls):
        setup_teardown_log.append("TestClassA.setup_class")
        cls.shared_resource = "A"
    
    @classmethod
    def teardown_class(cls):
        setup_teardown_log.append("TestClassA.teardown_class")
        # Clean up shared resource
        if hasattr(cls, 'shared_resource'):
            del cls.shared_resource
    
    def test_a1(self):
        assert self.shared_resource == "A"
        setup_teardown_log.append("TestClassA.test_a1")
    
    def test_a2(self):
        assert self.shared_resource == "A"
        setup_teardown_log.append("TestClassA.test_a2")


class TestClassB:
    @classmethod
    def setup_class(cls):
        setup_teardown_log.append("TestClassB.setup_class")
        cls.shared_resource = "B"
    
    @classmethod
    def teardown_class(cls):
        setup_teardown_log.append("TestClassB.teardown_class")
        # Clean up shared resource
        if hasattr(cls, 'shared_resource'):
            del cls.shared_resource
    
    def test_b1(self):
        assert self.shared_resource == "B"
        setup_teardown_log.append("TestClassB.test_b1")
    
    def test_b2(self):
        assert self.shared_resource == "B"
        setup_teardown_log.append("TestClassB.test_b2")


def test_module_level():
    """Module level test that should run after classes"""
    setup_teardown_log.append("test_module_level")
    # At this point, both classes should have been torn down
    expected_order = [
        "TestClassA.setup_class",
        "TestClassA.test_a1",
        "TestClassA.test_a2",
        "TestClassA.teardown_class",  # Should be called before TestClassB starts
        "TestClassB.setup_class",
        "TestClassB.test_b1", 
        "TestClassB.test_b2",
        "TestClassB.teardown_class",  # Should be called before module-level test
        "test_module_level"
    ]
    
    # Print actual order for debugging
    print("Actual order:", setup_teardown_log)
    print("Expected order:", expected_order)
    
    # Allow for some flexibility in test execution order within a class
    # but ensure setup/teardown happen at the right times
    assert "TestClassA.setup_class" in setup_teardown_log
    assert "TestClassA.teardown_class" in setup_teardown_log
    assert "TestClassB.setup_class" in setup_teardown_log
    assert "TestClassB.teardown_class" in setup_teardown_log
    
    # Key assertions: teardown_class should be called when transitioning
    a_teardown_idx = setup_teardown_log.index("TestClassA.teardown_class")
    b_setup_idx = setup_teardown_log.index("TestClassB.setup_class")
    assert a_teardown_idx < b_setup_idx, "TestClassA should be torn down before TestClassB setup"
    
    b_teardown_idx = setup_teardown_log.index("TestClassB.teardown_class")
    module_test_idx = setup_teardown_log.index("test_module_level")
    assert b_teardown_idx < module_test_idx, "TestClassB should be torn down before module-level test"