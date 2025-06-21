import pytest

# Unicode fixtures at module level
@pytest.fixture
def 日本語_データ():
    """Japanese fixture returning data"""
    return {"言語": "日本語", "挨拶": "こんにちは"}

@pytest.fixture
def 中文数据():
    """Chinese fixture returning data"""
    return {"语言": "中文", "问候": "你好"}

@pytest.fixture
def русские_данные():
    """Russian fixture returning data"""
    return {"язык": "русский", "приветствие": "привет"}

@pytest.fixture
def données_françaises():
    """French fixture with accented characters"""
    return {"langue": "français", "salutation": "bonjour"}

# Test using unicode fixtures
def test_using_japanese_fixture(日本語_データ):
    """Test using Japanese fixture"""
    assert 日本語_データ["言語"] == "日本語"
    assert 日本語_データ["挨拶"] == "こんにちは"

def test_using_chinese_fixture(中文数据):
    """Test using Chinese fixture"""
    assert 中文数据["语言"] == "中文"
    assert 中文数据["问候"] == "你好"

def test_using_russian_fixture(русские_данные):
    """Test using Russian fixture"""
    assert русские_данные["язык"] == "русский"
    assert русские_данные["приветствие"] == "привет"

def test_using_french_fixture(données_françaises):
    """Test using French fixture with accents"""
    assert données_françaises["langue"] == "français"
    assert données_françaises["salutation"] == "bonjour"

# Multiple unicode fixtures in one test
def test_multiple_unicode_fixtures(日本語_データ, 中文数据, русские_данные):
    """Test using multiple unicode fixtures"""
    assert 日本語_データ["言語"] == "日本語"
    assert 中文数据["语言"] == "中文"
    assert русские_данные["язык"] == "русский"

# Class-scoped unicode fixtures
class Test유니코드_클래스:
    """Korean class name with unicode fixtures"""
    
    @pytest.fixture
    def 한글_fixture(self):
        """Korean fixture"""
        return {"언어": "한국어", "인사": "안녕하세요"}
    
    def test_한글_fixture_사용(self, 한글_fixture):
        """Test using Korean fixture"""
        assert 한글_fixture["언어"] == "한국어"
        assert 한글_fixture["인사"] == "안녕하세요"

# Parametrized unicode fixtures
@pytest.fixture(params=["你好", "こんにちは", "안녕하세요", "مرحبا"])
def greeting_fixture(request):
    """Fixture with unicode parameters"""
    return {"greeting": request.param, "length": len(request.param)}

def test_parametrized_unicode_fixture(greeting_fixture):
    """Test parametrized fixture with unicode values"""
    assert isinstance(greeting_fixture["greeting"], str)
    assert greeting_fixture["length"] > 0

# Unicode fixture with indirect parametrization
@pytest.fixture
def translate_fixture(request):
    """Fixture that translates based on parameter"""
    translations = {
        "hello": "你好",
        "goodbye": "再见",
        "thanks": "谢谢",
        "sorry": "对不起"
    }
    return translations.get(request.param, request.param)

@pytest.mark.parametrize("translate_fixture", ["hello", "goodbye", "thanks"], indirect=True)
def test_indirect_unicode_fixture(translate_fixture):
    """Test indirect parametrization returning unicode"""
    assert translate_fixture in ["你好", "再见", "谢谢"]

# Yield fixtures with unicode
@pytest.fixture
def διαχειριστής_πόρων():
    """Greek yield fixture (resource manager)"""
    # Setup
    resource = {"κατάσταση": "ανοιχτό"}
    yield resource
    # Teardown
    resource["κατάσταση"] = "κλειστό"

def test_greek_yield_fixture(διαχειριστής_πόρων):
    """Test Greek yield fixture"""
    assert διαχειριστής_πόρων["κατάσταση"] == "ανοιχτό"

# Autouse unicode fixtures
@pytest.fixture(autouse=True)
def אתחול_אוטומטי():
    """Hebrew autouse fixture"""
    # This runs automatically before each test
    return {"מצב": "מאותחל"}

def test_hebrew_autouse():
    """Test that Hebrew autouse fixture runs"""
    # The fixture runs automatically
    assert True

# Module-scoped unicode fixture
@pytest.fixture(scope="module")
def বাংলা_module_fixture():
    """Bengali module-scoped fixture"""
    return {"ভাষা": "বাংলা", "স্বাগত": "স্বাগতম"}

def test_bengali_fixture_1(বাংলা_module_fixture):
    """First test using Bengali fixture"""
    assert বাংলা_module_fixture["ভাষা"] == "বাংলা"

def test_bengali_fixture_2(বাংলা_module_fixture):
    """Second test using same Bengali fixture instance"""
    assert বাংলা_module_fixture["স্বাগত"] == "স্বাগতম"

# Complex unicode fixture dependencies
@pytest.fixture
def base_datos():
    """Spanish base fixture"""
    return {"tipo": "base", "idioma": "español"}

@pytest.fixture
def fixture_derivada(base_datos):
    """Derived fixture depending on Spanish base"""
    base_datos["nivel"] = "derivado"
    return base_datos

def test_spanish_fixture_dependency(fixture_derivada):
    """Test Spanish fixture with dependencies"""
    assert fixture_derivada["tipo"] == "base"
    assert fixture_derivada["idioma"] == "español"
    assert fixture_derivada["nivel"] == "derivado"

# Unicode in fixture error messages
@pytest.fixture
def fixture_con_error():
    """Fixture that might raise with unicode message"""
    def _inner(should_fail=False):
        if should_fail:
            raise ValueError("Error con mensaje en español: ¡Algo salió mal!")
        return "éxito"
    return _inner

def test_unicode_error_fixture(fixture_con_error):
    """Test fixture with potential unicode error"""
    assert fixture_con_error(False) == "éxito"
    
    with pytest.raises(ValueError) as excinfo:
        fixture_con_error(True)
    assert "español" in str(excinfo.value)

# Mixed ASCII and unicode fixture names
@pytest.fixture
def mixed_fixture_名前():
    """Mixed ASCII and Japanese fixture name"""
    return "mixed content 混合内容"

def test_mixed_name_fixture(mixed_fixture_名前):
    """Test fixture with mixed ASCII/unicode name"""
    assert "mixed" in mixed_fixture_名前
    assert "混合" in mixed_fixture_名前

# Unicode fixture with special characters
@pytest.fixture
def café_fixture():
    """Fixture with accented character"""
    return {"name": "café", "price": "2.50€"}

def test_cafe_fixture(café_fixture):
    """Test fixture with accented name"""
    assert café_fixture["name"] == "café"
    assert "€" in café_fixture["price"]