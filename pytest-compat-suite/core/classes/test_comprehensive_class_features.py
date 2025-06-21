"""
Comprehensive test of all class-based features implemented
"""

class TestBasicClassMethods:
    """Test basic class method execution"""
    
    def test_simple_assertion(self):
        """Simple assertion test"""
        assert True
    
    def test_arithmetic(self):
        """Test arithmetic operations"""
        assert 2 + 2 == 4
    
    def test_string_operations(self):
        """Test string operations"""
        text = "fastest"
        assert text.upper() == "FASTEST"

class TestClassWithSetup:
    """Test class with setup methods"""
    
    @classmethod
    def setup_class(cls):
        """Class-level setup"""
        cls.shared_data = "shared_value"
        cls.counter = 0
    
    def setUp(self):
        """Instance-level setup"""
        self.instance_data = "instance_value"
        TestClassWithSetup.counter += 1
    
    def test_shared_data_access(self):
        """Test access to shared class data"""
        assert self.shared_data == "shared_value"
        assert hasattr(self, 'instance_data')
        assert self.instance_data == "instance_value"
    
    def test_counter_increment(self):
        """Test that setup is called for each test"""
        assert TestClassWithSetup.counter >= 1

class TestClassWithFixtures:
    """Test class with fixture usage"""
    
    def test_with_tmp_path(self, tmp_path):
        """Test using tmp_path fixture"""
        test_file = tmp_path / "test.txt"
        test_file.write_text("fastest")
        assert test_file.read_text() == "fastest"
    
    def test_with_monkeypatch(self, monkeypatch):
        """Test using monkeypatch fixture"""
        import os
        monkeypatch.setattr(os, 'getcwd', lambda: '/mocked/path')
        assert os.getcwd() == '/mocked/path'

class TestInheritancePattern(TestBasicClassMethods):
    """Test class inheritance"""
    
    def test_inherited_functionality(self):
        """Test that inheritance works properly"""
        assert True
    
    def test_override_behavior(self):
        """Test overriding parent behavior"""
        assert 3 + 3 == 6

# Function test for comparison
def test_function_level():
    """Function-level test for comparison"""
    assert True