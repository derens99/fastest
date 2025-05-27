#!/usr/bin/env python3
"""
Quickstart script for testing fastest in a new project
"""
import os
import subprocess
import sys
from pathlib import Path

def create_test_project():
    """Create a sample project for testing fastest"""
    
    print("🚀 Creating sample project for testing fastest...")
    
    # Create project directory
    project_dir = Path("fastest-demo")
    project_dir.mkdir(exist_ok=True)
    
    # Create test files
    test_files = {
        "test_math.py": '''
def test_addition():
    assert 1 + 1 == 2

def test_subtraction():
    assert 5 - 3 == 2

def test_multiplication():
    assert 3 * 4 == 12

def test_division():
    assert 10 / 2 == 5

class TestMathOperations:
    def test_power(self):
        assert 2 ** 3 == 8
    
    def test_modulo(self):
        assert 10 % 3 == 1
''',
        "test_strings.py": '''
def test_upper():
    assert "hello".upper() == "HELLO"

def test_lower():
    assert "WORLD".lower() == "world"

def test_capitalize():
    assert "python".capitalize() == "Python"

def test_strip():
    assert "  spaces  ".strip() == "spaces"

class TestStringMethods:
    def test_split(self):
        assert "a,b,c".split(",") == ["a", "b", "c"]
    
    def test_join(self):
        assert "-".join(["a", "b", "c"]) == "a-b-c"
''',
        "test_lists.py": '''
def test_append():
    lst = [1, 2, 3]
    lst.append(4)
    assert lst == [1, 2, 3, 4]

def test_extend():
    lst = [1, 2]
    lst.extend([3, 4])
    assert lst == [1, 2, 3, 4]

def test_insert():
    lst = [1, 3]
    lst.insert(1, 2)
    assert lst == [1, 2, 3]

def test_remove():
    lst = [1, 2, 3, 2]
    lst.remove(2)
    assert lst == [1, 3, 2]

class TestListComprehensions:
    def test_squares(self):
        squares = [x**2 for x in range(5)]
        assert squares == [0, 1, 4, 9, 16]
    
    def test_filter(self):
        evens = [x for x in range(10) if x % 2 == 0]
        assert evens == [0, 2, 4, 6, 8]
''',
        "pytest.ini": '''[tool:pytest]
# Test discovery patterns
python_files = test_*.py
python_classes = Test*
python_functions = test_*

# Fastest configuration
fastest_optimizer = lightning
fastest_workers = 4
'''
    }
    
    # Create test files
    tests_dir = project_dir / "tests"
    tests_dir.mkdir(exist_ok=True)
    
    for filename, content in test_files.items():
        if filename == "pytest.ini":
            filepath = project_dir / filename
        else:
            filepath = tests_dir / filename
        
        with open(filepath, 'w') as f:
            f.write(content.strip())
        print(f"  ✓ Created {filepath}")
    
    print(f"\n✅ Sample project created in '{project_dir}/'")
    print("\nTo test fastest:")
    print(f"  cd {project_dir}")
    print("  fastest tests/")
    print("  fastest tests/ --optimizer simple")
    print("  fastest tests/ -v")
    
    return project_dir

def run_benchmarks(project_dir):
    """Run benchmarks comparing fastest and pytest"""
    
    print("\n📊 Running performance comparison...")
    os.chdir(project_dir)
    
    # Run with pytest
    print("\n1. Running with pytest:")
    try:
        result = subprocess.run(
            ["python3", "-m", "pytest", "tests/", "-q"],
            capture_output=True,
            text=True
        )
        print(result.stdout)
        if result.stderr:
            print(result.stderr)
    except Exception as e:
        print(f"  ⚠️  pytest not installed: {e}")
    
    # Run with fastest
    print("\n2. Running with fastest:")
    optimizers = ["simple", "lightning", "optimized"]
    for optimizer in optimizers:
        print(f"\n  Optimizer: {optimizer}")
        try:
            result = subprocess.run(
                ["fastest", "tests/", "--optimizer", optimizer],
                capture_output=True,
                text=True
            )
            print(result.stdout)
            if result.stderr:
                print(result.stderr)
        except FileNotFoundError:
            print("  ⚠️  fastest not found. Please install it first:")
            print("     ./install-dev.sh")
            break
        except Exception as e:
            print(f"  Error: {e}")

def main():
    """Main function"""
    print("Fastest Test Runner - Quick Start Demo")
    print("=" * 50)
    
    # Check if fastest is installed
    try:
        subprocess.run(["fastest", "--version"], 
                      capture_output=True, check=True)
        print("✓ fastest is installed")
    except (subprocess.CalledProcessError, FileNotFoundError):
        print("⚠️  fastest not found. Installing...")
        print("\nPlease run: ./install-dev.sh")
        print("Then run this script again.")
        sys.exit(1)
    
    # Create test project
    project_dir = create_test_project()
    
    # Ask if user wants to run benchmarks
    response = input("\nRun performance comparison? (y/N): ")
    if response.lower() == 'y':
        run_benchmarks(project_dir)
    
    print("\n🎉 Done! You can now test fastest in the 'fastest-demo' directory.")

if __name__ == "__main__":
    main()