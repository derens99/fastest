"""Simple test file to verify basic class-based test functionality"""


class TestSimpleMath:
    """Simple test class with basic tests"""
    
    def test_addition(self):
        """Test simple addition"""
        assert 1 + 1 == 2
        assert 2 + 3 == 5
    
    def test_subtraction(self):
        """Test simple subtraction"""
        assert 5 - 3 == 2
        assert 10 - 7 == 3
    
    def test_multiplication(self):
        """Test simple multiplication"""
        assert 2 * 3 == 6
        assert 4 * 5 == 20


class TestStringOperations:
    """Test class for string operations"""
    
    def test_concatenation(self):
        """Test string concatenation"""
        assert "hello" + " " + "world" == "hello world"
    
    def test_upper_lower(self):
        """Test string case operations"""
        assert "hello".upper() == "HELLO"
        assert "WORLD".lower() == "world"
    
    def test_string_methods(self):
        """Test various string methods"""
        text = "  hello world  "
        assert text.strip() == "hello world"
        assert text.replace("world", "python") == "  hello python  "


class TestWithSetupTeardown:
    """Test class with setup and teardown"""
    
    def setUp(self):
        """Setup before each test"""
        self.test_list = [1, 2, 3]
        self.test_dict = {"key": "value"}
    
    def test_list_operations(self):
        """Test list operations using setup data"""
        self.test_list.append(4)
        assert len(self.test_list) == 4
        assert self.test_list[-1] == 4
    
    def test_dict_operations(self):
        """Test dict operations using setup data"""
        self.test_dict["new_key"] = "new_value"
        assert len(self.test_dict) == 2
        assert self.test_dict.get("new_key") == "new_value"


# Regular function tests for comparison
def test_function_simple():
    """Simple function-based test"""
    assert True


def test_function_with_math():
    """Function test with math"""
    assert 10 / 2 == 5
    assert 3 ** 2 == 9