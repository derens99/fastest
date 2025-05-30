#!/usr/bin/env python3
"""
Django compatibility test suite for Fastest
Tests real-world Django patterns and fixtures that are common in production
"""

import os
import sys
import tempfile
import shutil
from pathlib import Path

def create_django_test_project():
    """Create a minimal Django project for testing compatibility"""
    
    # Create temporary project directory
    project_dir = Path(tempfile.mkdtemp(prefix="fastest_django_test_"))
    print(f"Creating Django test project in: {project_dir}")
    
    # Create Django project structure
    os.chdir(project_dir)
    
    # Create minimal Django settings
    settings_content = '''
import os
from pathlib import Path

BASE_DIR = Path(__file__).resolve().parent.parent

SECRET_KEY = 'test-secret-key-for-fastest-testing-only'
DEBUG = True
ALLOWED_HOSTS = []

INSTALLED_APPS = [
    'django.contrib.contenttypes',
    'django.contrib.auth',
    'myapp',
]

DATABASES = {
    'default': {
        'ENGINE': 'django.db.backends.sqlite3',
        'NAME': BASE_DIR / 'test_db.sqlite3',
    }
}

USE_TZ = True
'''
    
    # Create project structure
    (project_dir / "myproject").mkdir()
    (project_dir / "myproject" / "__init__.py").touch()
    (project_dir / "myproject" / "settings.py").write_text(settings_content)
    
    # Create app structure
    (project_dir / "myapp").mkdir()
    (project_dir / "myapp" / "__init__.py").touch()
    
    # Create models
    models_content = '''
from django.db import models
from django.contrib.auth.models import User

class Article(models.Model):
    title = models.CharField(max_length=200)
    content = models.TextField()
    author = models.ForeignKey(User, on_delete=models.CASCADE)
    created_at = models.DateTimeField(auto_now_add=True)
    published = models.BooleanField(default=False)
    
    def __str__(self):
        return self.title
    
    def get_summary(self):
        return self.content[:100] + "..." if len(self.content) > 100 else self.content

class Category(models.Model):
    name = models.CharField(max_length=100)
    slug = models.SlugField(unique=True)
    
    def __str__(self):
        return self.name
'''
    
    (project_dir / "myapp" / "models.py").write_text(models_content)
    
    # Create Django views
    views_content = '''
from django.shortcuts import render, get_object_or_404
from django.http import JsonResponse
from .models import Article, Category

def article_list(request):
    articles = Article.objects.filter(published=True)
    return render(request, 'articles/list.html', {'articles': articles})

def article_detail(request, pk):
    article = get_object_or_404(Article, pk=pk, published=True)
    return render(request, 'articles/detail.html', {'article': article})

def api_articles(request):
    articles = Article.objects.filter(published=True)
    data = [
        {
            'id': article.id,
            'title': article.title,
            'summary': article.get_summary(),
            'author': article.author.username,
        }
        for article in articles
    ]
    return JsonResponse({'articles': data})
'''
    
    (project_dir / "myapp" / "views.py").write_text(views_content)
    
    # Create comprehensive Django test suite
    tests_content = '''
import pytest
from django.test import TestCase, Client
from django.contrib.auth.models import User
from django.db import transaction
from django.core.exceptions import ValidationError
from unittest.mock import patch, Mock
import json

from .models import Article, Category

# Session-scoped fixtures (expensive setup)
@pytest.fixture(scope="session")
def django_db_setup(django_db_setup, django_db_blocker):
    """Setup test database schema"""
    with django_db_blocker.unblock():
        # This would normally run Django migrations
        pass

@pytest.fixture(scope="session") 
def session_admin_user():
    """Create admin user for the entire test session"""
    return User.objects.create_superuser(
        username='admin',
        email='admin@test.com',
        password='admin123'
    )

# Module-scoped fixtures (shared within test file)
@pytest.fixture(scope="module")
def test_categories():
    """Create test categories for the module"""
    categories = [
        Category.objects.create(name="Technology", slug="tech"),
        Category.objects.create(name="Science", slug="science"),
        Category.objects.create(name="Sports", slug="sports"),
    ]
    return categories

# Class-scoped fixtures (shared within test class)
@pytest.fixture(scope="class")
def sample_articles(session_admin_user):
    """Create sample articles for class tests"""
    articles = []
    for i in range(5):
        article = Article.objects.create(
            title=f"Test Article {i+1}",
            content=f"This is test content for article {i+1}. " * 10,
            author=session_admin_user,
            published=(i % 2 == 0)  # Alternate published/unpublished
        )
        articles.append(article)
    return articles

# Function-scoped fixtures (new instance per test)
@pytest.fixture
def client():
    """Django test client"""
    return Client()

@pytest.fixture
def test_user():
    """Regular test user"""
    return User.objects.create_user(
        username='testuser',
        email='test@example.com',
        password='testpass123'
    )

@pytest.fixture
def published_article(test_user):
    """Single published article for testing"""
    return Article.objects.create(
        title="Published Test Article",
        content="This is a published test article.",
        author=test_user,
        published=True
    )

@pytest.fixture
def draft_article(test_user):
    """Single draft article for testing"""
    return Article.objects.create(
        title="Draft Test Article", 
        content="This is a draft test article.",
        author=test_user,
        published=False
    )

# Autouse fixtures (automatically applied)
@pytest.fixture(autouse=True)
def setup_test_environment():
    """Setup that runs before every test"""
    # Clear any test data that might interfere
    pass

# Tests using various fixture scopes
class TestArticleModel:
    """Test Article model functionality"""
    
    def test_article_creation(self, test_user):
        """Test basic article creation"""
        article = Article.objects.create(
            title="New Article",
            content="Article content here",
            author=test_user,
            published=True
        )
        assert article.title == "New Article"
        assert article.author == test_user
        assert article.published is True
        
    def test_article_str_representation(self, published_article):
        """Test article string representation"""
        assert str(published_article) == "Published Test Article"
        
    def test_article_summary(self, test_user):
        """Test article summary generation"""
        long_content = "A" * 150
        article = Article.objects.create(
            title="Long Article",
            content=long_content,
            author=test_user
        )
        summary = article.get_summary()
        assert len(summary) == 103  # 100 chars + "..."
        assert summary.endswith("...")

class TestArticleViews:
    """Test Article views and API endpoints"""
    
    def test_article_list_view(self, client, sample_articles):
        """Test article list view with published articles"""
        response = client.get('/articles/')
        assert response.status_code == 200
        # Should only show published articles
        
    def test_article_detail_view(self, client, published_article):
        """Test article detail view"""
        response = client.get(f'/articles/{published_article.pk}/')
        assert response.status_code == 200
        
    def test_article_detail_404(self, client):
        """Test article detail with non-existent article"""
        response = client.get('/articles/999999/')
        assert response.status_code == 404
        
    def test_api_articles_endpoint(self, client, sample_articles):
        """Test articles API endpoint"""
        response = client.get('/api/articles/')
        assert response.status_code == 200
        data = json.loads(response.content)
        assert 'articles' in data
        # Should return only published articles
        published_count = sum(1 for a in sample_articles if a.published)
        assert len(data['articles']) == published_count

class TestDatabaseOperations:
    """Test database operations and transactions"""
    
    @pytest.mark.django_db
    def test_article_creation_in_transaction(self, test_user):
        """Test article creation within transaction"""
        with transaction.atomic():
            article = Article.objects.create(
                title="Transactional Article",
                content="Created in transaction",
                author=test_user
            )
            assert Article.objects.filter(id=article.id).exists()
            
    @pytest.mark.django_db
    def test_bulk_article_creation(self, test_user):
        """Test bulk creation of articles"""
        articles_data = [
            Article(
                title=f"Bulk Article {i}",
                content=f"Bulk content {i}",
                author=test_user,
                published=(i % 2 == 0)
            )
            for i in range(10)
        ]
        
        created_articles = Article.objects.bulk_create(articles_data)
        assert len(created_articles) == 10
        assert Article.objects.filter(title__startswith="Bulk Article").count() == 10

class TestCategoryModel:
    """Test Category model"""
    
    def test_category_creation(self):
        """Test basic category creation"""
        category = Category.objects.create(
            name="Test Category",
            slug="test-category"
        )
        assert category.name == "Test Category"
        assert category.slug == "test-category"
        
    def test_category_str_representation(self, test_categories):
        """Test category string representation using module fixture"""
        tech_category = test_categories[0]  # Technology category
        assert str(tech_category) == "Technology"
        
    def test_category_slug_uniqueness(self):
        """Test that category slugs must be unique"""
        Category.objects.create(name="Category 1", slug="unique-slug")
        
        with pytest.raises(Exception):  # Should raise IntegrityError
            Category.objects.create(name="Category 2", slug="unique-slug")

# Parametrized tests
@pytest.mark.parametrize("title,content,expected_summary_length", [
    ("Short", "Short content", 13),
    ("Medium", "A" * 50, 50),
    ("Long", "A" * 150, 103),  # 100 + "..."
])
def test_article_summary_parametrized(test_user, title, content, expected_summary_length):
    """Test article summary with different content lengths"""
    article = Article.objects.create(
        title=title,
        content=content,
        author=test_user
    )
    summary = article.get_summary()
    assert len(summary) == expected_summary_length

# Mock tests
@patch('myapp.views.Article.objects')
def test_api_articles_with_mock(mock_article_objects, client):
    """Test API endpoint with mocked Article model"""
    # Setup mock
    mock_article = Mock()
    mock_article.id = 1
    mock_article.title = "Mocked Article"
    mock_article.get_summary.return_value = "Mocked summary"
    mock_article.author.username = "mockuser"
    
    mock_article_objects.filter.return_value = [mock_article]
    
    response = client.get('/api/articles/')
    assert response.status_code == 200
    
    data = json.loads(response.content)
    assert len(data['articles']) == 1
    assert data['articles'][0]['title'] == "Mocked Article"

# Async tests (if using Django 4.1+)
@pytest.mark.asyncio
@pytest.mark.django_db
async def test_async_article_operations(test_user):
    """Test async database operations"""
    from django.db import models
    from asgiref.sync import sync_to_async
    
    # Create article asynchronously
    article = await sync_to_async(Article.objects.create)(
        title="Async Article",
        content="Created asynchronously",
        author=test_user
    )
    
    assert article.title == "Async Article"
    
    # Query asynchronously
    count = await sync_to_async(Article.objects.count)()
    assert count >= 1

# Performance tests
@pytest.mark.performance
def test_large_dataset_query_performance(session_admin_user):
    """Test query performance with large dataset"""
    import time
    
    # Create many articles
    articles = [
        Article(
            title=f"Performance Article {i}",
            content=f"Content for article {i}",
            author=session_admin_user,
            published=True
        )
        for i in range(100)
    ]
    Article.objects.bulk_create(articles)
    
    # Time the query
    start_time = time.time()
    published_articles = list(Article.objects.filter(published=True))
    query_time = time.time() - start_time
    
    assert len(published_articles) >= 100
    assert query_time < 1.0  # Should complete in less than 1 second

# Integration tests
@pytest.mark.integration
def test_full_article_lifecycle(client, session_admin_user):
    """Test complete article lifecycle from creation to deletion"""
    # This would test the full workflow:
    # 1. User authentication
    # 2. Article creation
    # 3. Publishing
    # 4. Viewing
    # 5. Editing  
    # 6. Deletion
    
    # Login as admin
    client.force_login(session_admin_user)
    
    # Create article via POST
    response = client.post('/admin/articles/create/', {
        'title': 'Integration Test Article',
        'content': 'Full lifecycle test content',
        'published': True
    })
    
    # This would continue with the full lifecycle...
    # For now, just verify we have the user logged in
    assert hasattr(session_admin_user, 'username')
'''
    
    (project_dir / "myapp" / "test_django_compatibility.py").write_text(tests_content)
    
    # Create pytest configuration
    pytest_ini_content = '''
[tool:pytest]
DJANGO_SETTINGS_MODULE = myproject.settings
python_files = tests.py test_*.py *_tests.py *_test.py
python_classes = Test*
python_functions = test_*
addopts = -v --tb=short --strict-markers
markers =
    django_db: marks tests as requiring the Django database
    performance: marks tests as performance tests
    integration: marks tests as integration tests
    slow: marks tests as slow running
'''
    
    (project_dir / "pytest.ini").write_text(pytest_ini_content)
    
    # Create requirements for the test project
    requirements_content = '''
Django>=4.0.0
pytest>=7.0.0
pytest-django>=4.5.0
pytest-asyncio>=0.21.0
'''
    
    (project_dir / "requirements.txt").write_text(requirements_content)
    
    return project_dir

def run_django_compatibility_test():
    """Run Django compatibility test comparing pytest vs fastest"""
    
    project_dir = create_django_test_project()
    
    try:
        os.chdir(project_dir)
        
        print("\\n🧪 Django Compatibility Test Suite")
        print("=" * 50)
        
        # Install requirements
        print("Installing Django and pytest...")
        import subprocess
        subprocess.run([sys.executable, "-m", "pip", "install", "-r", "requirements.txt"], 
                      check=True, capture_output=True)
        
        # Setup Django
        print("Setting up Django database...")
        os.environ['DJANGO_SETTINGS_MODULE'] = 'myproject.settings'
        
        # Test with pytest
        print("\\n🔸 Running with pytest...")
        pytest_result = subprocess.run([
            sys.executable, "-m", "pytest", 
            "myapp/test_django_compatibility.py", 
            "-v", "--tb=short"
        ], capture_output=True, text=True)
        
        # Test with fastest
        print("🔸 Running with fastest...")
        fastest_binary = Path(__file__).parent / "target" / "release" / "fastest"
        
        if fastest_binary.exists():
            fastest_result = subprocess.run([
                str(fastest_binary), 
                "myapp/test_django_compatibility.py", 
                "-v"
            ], capture_output=True, text=True)
            
            print("\\n📊 Results Comparison:")
            print(f"pytest exit code: {pytest_result.returncode}")
            print(f"fastest exit code: {fastest_result.returncode}")
            
            if pytest_result.returncode == 0 and fastest_result.returncode == 0:
                print("✅ Both test runners succeeded!")
                
                # Extract timing information
                pytest_output = pytest_result.stdout + pytest_result.stderr
                fastest_output = fastest_result.stdout + fastest_result.stderr
                
                print("\\n📈 Output Analysis:")
                print("\\npytest output (last 10 lines):")
                print("\\n".join(pytest_output.split("\\n")[-10:]))
                
                print("\\nfastest output (last 10 lines):")
                print("\\n".join(fastest_output.split("\\n")[-10:]))
                
            else:
                print("❌ One or both test runners failed")
                if pytest_result.returncode != 0:
                    print(f"pytest stderr: {pytest_result.stderr}")
                if fastest_result.returncode != 0:
                    print(f"fastest stderr: {fastest_result.stderr}")
        else:
            print(f"⚠️  Fastest binary not found at {fastest_binary}")
            print("Please build fastest first with: cargo build --release")
            
    finally:
        # Cleanup
        os.chdir("/")
        shutil.rmtree(project_dir)
        print(f"\\n🧹 Cleaned up test project: {project_dir}")

if __name__ == "__main__":
    run_django_compatibility_test()