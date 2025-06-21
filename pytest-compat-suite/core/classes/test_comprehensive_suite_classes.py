"""
Comprehensive Test Suite - Class-based Tests
Tests for class-based testing with inheritance and setup/teardown
"""

import pytest


# Basic test class
class TestBasicClass:
    """Basic test class with simple test methods"""
    
    def test_method_1(self):
        """First test method"""
        assert True
    
    def test_method_2(self):
        """Second test method"""
        assert 1 + 1 == 2
    
    def test_method_3(self):
        """Third test method"""
        assert "hello".upper() == "HELLO"


# Class with setup and teardown
class TestWithSetupTeardown:
    """Test class with setup and teardown methods"""
    
    def setup_method(self, method):
        """Called before each test method"""
        self.test_data = {"setup": True}
        self.method_name = method.__name__
        print(f"Setting up {self.method_name}")
    
    def teardown_method(self, method):
        """Called after each test method"""
        print(f"Tearing down {method.__name__}")
        self.test_data = None
    
    def test_setup_was_called(self):
        """Test that setup_method was called"""
        assert hasattr(self, 'test_data')
        assert self.test_data["setup"] is True
        assert self.method_name == "test_setup_was_called"
    
    def test_isolated_setup(self):
        """Test that each test gets fresh setup"""
        assert self.test_data == {"setup": True}
        self.test_data["modified"] = True
    
    def test_no_bleed_between_tests(self):
        """Test that modifications don't bleed between tests"""
        assert self.test_data == {"setup": True}
        assert "modified" not in self.test_data


# Class with class-level setup/teardown
class TestWithClassSetupTeardown:
    """Test class with class-level setup and teardown"""
    
    @classmethod
    def setup_class(cls):
        """Called once for the entire class"""
        cls.shared_resource = {"class_setup": True, "counter": 0}
        print("Setting up TestWithClassSetupTeardown class")
    
    @classmethod
    def teardown_class(cls):
        """Called once after all tests in class"""
        print("Tearing down TestWithClassSetupTeardown class")
        cls.shared_resource = None
    
    def test_class_setup_1(self):
        """First test using class setup"""
        assert self.shared_resource["class_setup"] is True
        self.shared_resource["counter"] += 1
    
    def test_class_setup_2(self):
        """Second test sees changes from first test"""
        assert self.shared_resource["counter"] >= 1
        self.shared_resource["counter"] += 1
    
    def test_class_setup_3(self):
        """Third test sees accumulated changes"""
        assert self.shared_resource["counter"] >= 2


# Class with both method and class setup/teardown
class TestWithBothSetups:
    """Test class with both levels of setup/teardown"""
    
    @classmethod
    def setup_class(cls):
        """Class-level setup"""
        cls.class_data = {"type": "shared"}
    
    @classmethod
    def teardown_class(cls):
        """Class-level teardown"""
        cls.class_data = None
    
    def setup_method(self, method):
        """Method-level setup"""
        self.method_data = {"type": "per-method"}
    
    def teardown_method(self, method):
        """Method-level teardown"""
        self.method_data = None
    
    def test_both_setups(self):
        """Test that has access to both setup levels"""
        assert self.class_data["type"] == "shared"
        assert self.method_data["type"] == "per-method"
    
    def test_isolation(self):
        """Test proper isolation between levels"""
        self.method_data["modified"] = True
        self.class_data["modified"] = True
    
    def test_method_isolation_maintained(self):
        """Test that method data is isolated"""
        assert "modified" not in self.method_data
        assert "modified" in self.class_data  # Class data persists


# Class inheritance
class BaseTestClass:
    """Base class for test inheritance"""
    
    def setup_method(self, method):
        """Base setup method"""
        self.base_data = {"base": True}
    
    def test_base_method(self):
        """Test method in base class"""
        assert self.base_data["base"] is True
    
    def test_to_override(self):
        """Method that will be overridden"""
        assert False, "Should be overridden"


class TestInheritance(BaseTestClass):
    """Test class that inherits from base"""
    
    def setup_method(self, method):
        """Override setup - should call super()"""
        super().setup_method(method)
        self.derived_data = {"derived": True}
    
    def test_inherited_method(self):
        """Test that inherited test method works"""
        # This inherits test_base_method which should work
        pass
    
    def test_to_override(self):
        """Override the base test method"""
        assert True  # Now it passes
    
    def test_derived_setup(self):
        """Test that both setups were called"""
        assert self.base_data["base"] is True
        assert self.derived_data["derived"] is True


# Multiple inheritance
class MixinClass:
    """Mixin class with test helpers"""
    
    def assert_positive(self, value):
        """Helper method for assertions"""
        assert value > 0
    
    def get_test_data(self):
        """Provide test data"""
        return [1, 2, 3, 4, 5]


class TestMultipleInheritance(MixinClass, BaseTestClass):
    """Test class with multiple inheritance"""
    
    def test_mixin_methods(self):
        """Test using mixin methods"""
        data = self.get_test_data()
        for value in data:
            self.assert_positive(value)
    
    def test_both_inheritances(self):
        """Test that both parent classes work"""
        self.base_data = {"test": True}  # From BaseTestClass
        self.assert_positive(5)  # From MixinClass


# unittest-style setup/teardown
class TestUnittestStyle:
    """Test class with unittest-style setup/teardown"""
    
    def setUp(self):
        """unittest-style setup (capital U)"""
        self.data = {"unittest": True}
    
    def tearDown(self):
        """unittest-style teardown (capital D)"""
        self.data = None
    
    def test_unittest_setup(self):
        """Test that unittest-style setup works"""
        assert self.data["unittest"] is True


# Class with fixtures
class TestClassWithFixtures:
    """Test class that uses fixtures"""
    
    @pytest.fixture(autouse=True)
    def setup_data(self):
        """Autouse fixture for class"""
        self.fixture_data = {"fixture": True}
        yield
        self.fixture_data = None
    
    def test_fixture_setup(self):
        """Test that fixture ran"""
        assert self.fixture_data["fixture"] is True
    
    def test_fixture_with_param(self, tmp_path):
        """Test using both autouse and parameter fixtures"""
        assert self.fixture_data["fixture"] is True
        assert tmp_path.exists()


# Nested test classes
class TestOuterClass:
    """Outer test class"""
    
    @classmethod
    def setup_class(cls):
        cls.outer_data = {"level": "outer"}
    
    def test_outer_method(self):
        """Test in outer class"""
        assert self.outer_data["level"] == "outer"
    
    class TestInnerClass:
        """Nested inner test class"""
        
        @classmethod
        def setup_class(cls):
            cls.inner_data = {"level": "inner"}
        
        def test_inner_method(self):
            """Test in inner class"""
            assert self.inner_data["level"] == "inner"
        
        def test_no_outer_access(self):
            """Inner class shouldn't access outer data"""
            assert not hasattr(self, 'outer_data')


# Class with async methods
class TestAsyncMethods:
    """Test class with async test methods"""
    
    async def test_async_method_1(self):
        """Async test method"""
        import asyncio
        await asyncio.sleep(0.01)
        assert True
    
    async def test_async_method_2(self):
        """Another async test method"""
        import asyncio
        
        async def async_function():
            await asyncio.sleep(0.01)
            return "result"
        
        result = await async_function()
        assert result == "result"
    
    def test_sync_method(self):
        """Regular sync method in same class"""
        assert True


# Class with parametrized methods
class TestParametrizedMethods:
    """Test class with parametrized test methods"""
    
    @pytest.mark.parametrize("value", [1, 2, 3])
    def test_parametrized_method(self, value):
        """Parametrized method in class"""
        assert value in [1, 2, 3]
    
    @pytest.mark.parametrize("x,y", [(1, 2), (3, 4)])
    def test_multi_param_method(self, x, y):
        """Multiple parameter method"""
        assert x < y


# Class with markers
@pytest.mark.integration
class TestMarkedClass:
    """Entire class marked as integration"""
    
    def test_inherits_class_marker(self):
        """Method inherits class marker"""
        assert True
    
    @pytest.mark.slow
    def test_additional_marker(self):
        """Method with additional marker"""
        import time
        time.sleep(0.01)
        assert True


# Complex setup/teardown ordering
class TestSetupTeardownOrder:
    """Test to verify setup/teardown order"""
    
    order = []
    
    @classmethod
    def setup_class(cls):
        cls.order.append("setup_class")
    
    @classmethod
    def teardown_class(cls):
        cls.order.append("teardown_class")
    
    def setup_method(self, method):
        self.order.append("setup_method")
    
    def teardown_method(self, method):
        self.order.append("teardown_method")
    
    def test_order_1(self):
        """First test to establish order"""
        self.order.append("test_1")
        assert "setup_class" in self.order
        assert "setup_method" in self.order
    
    def test_order_2(self):
        """Second test to verify order"""
        self.order.append("test_2")
        # Should see: setup_class, setup_method, test_1, teardown_method, setup_method
        assert self.order.count("setup_method") >= 2
        assert self.order.count("teardown_method") >= 1


# Class with failing setup
class TestFailingSetup:
    """Test class where setup fails"""
    
    def setup_method(self, method):
        """Setup that fails"""
        raise ValueError("Setup failed intentionally")
    
    @pytest.mark.xfail(reason="Setup fails")
    def test_should_not_run(self):
        """This test should not run due to setup failure"""
        assert False


# Empty test class (should be discovered but have no tests)
class TestEmptyClass:
    """Empty test class - should be handled gracefully"""
    pass


# Class not following naming convention (should not be discovered)
class NotATestClass:
    """This class should not be discovered as tests"""
    
    def test_method(self):
        """Even with test method, class should not be discovered"""
        assert False