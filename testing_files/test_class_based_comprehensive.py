"""Comprehensive test file for class-based test discovery and execution"""
import pytest
import unittest
import asyncio


class TestBasicClassMethods:
    """Basic test class with various method types"""
    
    def test_simple_method(self):
        """Simple test method"""
        assert 1 + 1 == 2
    
    def test_with_assertion(self):
        """Test with assertion"""
        x = 10
        y = 20
        assert x + y == 30
    
    def test_multiple_assertions(self):
        """Test with multiple assertions"""
        assert True
        assert not False
        assert 1 < 2
    
    def not_a_test_method(self):
        """This should not be discovered as a test"""
        return "helper"
    
    async def test_async_method(self):
        """Async test method"""
        await asyncio.sleep(0.001)
        assert True


class TestClassWithSetup:
    """Test class with setup and teardown methods"""
    
    def setUp(self):
        """Setup method - should be called before each test"""
        self.value = 42
        self.items = []
    
    def tearDown(self):
        """Teardown method - should be called after each test"""
        self.items.clear()
    
    def test_using_setup_value(self):
        """Test that uses value from setUp"""
        assert self.value == 42
        self.items.append(1)
    
    def test_modifying_setup_value(self):
        """Test that modifies setup value"""
        self.value = 100
        assert self.value == 100
        self.items.append(2)


class TestClassWithClassMethods:
    """Test class with class-level setup/teardown"""
    
    counter = 0
    
    @classmethod
    def setup_class(cls):
        """Class-level setup"""
        cls.shared_resource = "shared"
        cls.counter = 100
    
    @classmethod
    def teardown_class(cls):
        """Class-level teardown"""
        cls.shared_resource = None
        cls.counter = 0
    
    def test_using_class_resource(self):
        """Test using class-level resource"""
        assert self.shared_resource == "shared"
        assert self.counter == 100
    
    def test_incrementing_counter(self):
        """Test modifying class variable"""
        TestClassWithClassMethods.counter += 1
        assert TestClassWithClassMethods.counter == 101


class TestParametrizedMethods:
    """Test class with parametrized methods"""
    
    @pytest.mark.parametrize("x,y,expected", [
        (1, 2, 3),
        (5, 5, 10),
        (0, 0, 0),
        (-1, 1, 0),
    ])
    def test_parametrized_addition(self, x, y, expected):
        """Parametrized test method"""
        assert x + y == expected
    
    @pytest.mark.parametrize("value", [1, 2, 3, 4, 5])
    def test_single_param(self, value):
        """Single parameter test"""
        assert value > 0
        assert value < 10


class TestWithFixtures:
    """Test class using pytest fixtures"""
    
    def test_with_tmp_path(self, tmp_path):
        """Test using tmp_path fixture"""
        test_file = tmp_path / "test.txt"
        test_file.write_text("hello")
        assert test_file.read_text() == "hello"
    
    def test_with_monkeypatch(self, monkeypatch):
        """Test using monkeypatch fixture"""
        import os
        monkeypatch.setenv("TEST_VAR", "test_value")
        assert os.environ.get("TEST_VAR") == "test_value"
    
    def test_with_capsys(self, capsys):
        """Test using capsys fixture"""
        print("Hello, test!")
        captured = capsys.readouterr()
        assert "Hello, test!" in captured.out


class TestWithMarkers:
    """Test class with pytest markers"""
    
    @pytest.mark.skip(reason="Demonstrating skip marker")
    def test_skip_this(self):
        """This test should be skipped"""
        assert False  # This should not run
    
    @pytest.mark.xfail(reason="Expected to fail")
    def test_expected_failure(self):
        """This test is expected to fail"""
        assert False
    
    @pytest.mark.skipif(False, reason="Should not skip")
    def test_conditional_skip(self):
        """This test should run"""
        assert True


class TestInheritance(TestBasicClassMethods):
    """Test class that inherits from another test class"""
    
    def test_in_child_class(self):
        """Test defined in child class"""
        assert 2 * 2 == 4
    
    def test_simple_method(self):
        """Override parent test method"""
        assert 1 + 1 == 2
        assert "overridden" == "overridden"


class TestComplexScenarios:
    """Test class with complex test scenarios"""
    
    def test_with_exception_handling(self):
        """Test with exception handling"""
        try:
            raise ValueError("test error")
        except ValueError as e:
            assert str(e) == "test error"
    
    def test_with_context_manager(self):
        """Test using context managers"""
        class TestContext:
            def __enter__(self):
                return self
            def __exit__(self, *args):
                pass
        
        with TestContext() as ctx:
            assert ctx is not None
    
    def test_with_nested_functions(self):
        """Test with nested function definitions"""
        def helper(x):
            return x * 2
        
        assert helper(5) == 10
        assert helper(0) == 0


# This class should NOT be discovered (doesn't start with Test)
class NotATestClass:
    def test_should_not_be_found(self):
        """This should not be discovered"""
        assert False


# Function-based tests should still work
def test_function_based():
    """Regular function-based test"""
    assert True


async def test_async_function():
    """Async function-based test"""
    await asyncio.sleep(0.001)
    assert True


@pytest.mark.parametrize("n", [1, 2, 3])
def test_parametrized_function(n):
    """Parametrized function test"""
    assert n > 0


if __name__ == "__main__":
    # For debugging - count expected tests
    test_classes = [
        TestBasicClassMethods,      # 3 tests
        TestClassWithSetup,         # 2 tests
        TestClassWithClassMethods,  # 2 tests
        TestParametrizedMethods,    # 4 + 5 = 9 tests
        TestWithFixtures,           # 3 tests
        TestWithMarkers,            # 3 tests (including skipped ones)
        TestInheritance,            # 2 tests (inherited tests might not be counted)
        TestComplexScenarios,       # 3 tests
    ]
    
    function_tests = 3  # test_function_based, test_async_function, test_parametrized_function[0,1,2]
    
    print(f"Expected class-based tests: ~27")
    print(f"Expected function-based tests: {function_tests}")
    print(f"Total expected: ~30+ tests")