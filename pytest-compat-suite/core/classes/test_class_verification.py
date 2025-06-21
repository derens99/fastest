"""Verification test for class-based test discovery and execution"""


class TestClassDiscovery:
    """Verify that class-based test discovery works"""
    
    def test_method_discovery(self):
        """This method should be discovered"""
        assert True
    
    def test_another_method(self):
        """Another test method"""
        assert 1 == 1
    
    def helper_method(self):
        """This should NOT be discovered as a test"""
        return "I'm a helper"
    
    def testMethodWithoutUnderscore(self):
        """Methods starting with 'test' should also be discovered"""
        assert True


class TestAsyncMethods:
    """Test class with async methods"""
    
    async def test_async_method(self):
        """Async test method"""
        import asyncio
        await asyncio.sleep(0.001)
        assert True
    
    async def test_another_async(self):
        """Another async test"""
        result = await self.async_helper()
        assert result == 42
    
    async def async_helper(self):
        """Helper async method"""
        return 42


class TestInheritedMethods:
    """Base test class"""
    
    def test_base_method(self):
        """Method in base class"""
        assert True


class TestChildClass(TestInheritedMethods):
    """Child test class that inherits"""
    
    def test_child_method(self):
        """Method in child class"""
        assert True


# Class that should NOT be discovered
class NotTestClass:
    def test_not_discovered(self):
        """This should not be discovered"""
        assert False


class MyTestClass:
    """Class starting with different prefix - should NOT be discovered by pytest convention"""
    def test_also_not_discovered(self):
        """This should not be discovered"""
        assert False