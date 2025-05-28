#!/usr/bin/env python3
"""
Setup script for fastest-runner.

This provides backward compatibility for older Python packaging tools.
For modern installations, use pyproject.toml with pip or maturin.
"""

from setuptools import setup, find_packages
import os
import sys

# Ensure we're using Python 3.8+
if sys.version_info < (3, 8):
    print("Error: fastest requires Python 3.8 or later", file=sys.stderr)
    sys.exit(1)

# Read README for long description
def read_readme():
    readme_path = os.path.join(os.path.dirname(__file__), "README.md")
    if os.path.exists(readme_path):
        with open(readme_path, "r", encoding="utf-8") as f:
            return f.read()
    return "⚡ The blazing fast Python test runner with intelligent performance optimization"

# Read version from pyproject.toml or use default
def get_version():
    try:
        import tomli
        with open("pyproject.toml", "rb") as f:
            data = tomli.load(f)
            return data["project"]["version"]
    except:
        return "0.2.0"

setup(
    name="fastest-runner",
    version=get_version(),
    description="⚡ The blazing fast Python test runner with intelligent performance optimization",
    long_description=read_readme(),
    long_description_content_type="text/markdown",
    
    author="Fastest Team",
    author_email="hello@fastest.dev",
    
    url="https://github.com/derens99/fastest",
    project_urls={
        "Documentation": "https://github.com/derens99/fastest/tree/main/docs",
        "Repository": "https://github.com/derens99/fastest",
        "Issues": "https://github.com/derens99/fastest/issues",
        "Changelog": "https://github.com/derens99/fastest/blob/main/CHANGELOG.md",
        "Roadmap": "https://github.com/derens99/fastest/blob/main/ROADMAP.md",
        "Benchmarks": "https://github.com/derens99/fastest/tree/main/benchmarks",
    },
    
    packages=find_packages("python"),
    package_dir={"": "python"},
    
    python_requires=">=3.8",
    
    classifiers=[
        "Development Status :: 4 - Beta",
        "Intended Audience :: Developers",
        "License :: OSI Approved :: MIT License",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9", 
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Python :: 3.12",
        "Programming Language :: Rust",
        "Topic :: Software Development :: Testing",
        "Topic :: Software Development :: Testing :: Unit",
        "Topic :: Software Development :: Quality Assurance",
        "Environment :: Console",
        "Operating System :: POSIX",
        "Operating System :: MacOS :: MacOS X",
        "Operating System :: Microsoft :: Windows",
    ],
    
    keywords=[
        "testing", "test-runner", "pytest", "performance", "rust", 
        "parallel", "fast", "python", "benchmark", "optimization"
    ],
    
    entry_points={
        "console_scripts": [
            "fastest=fastest_runner:main",
        ],
    },
    
    extras_require={
        "dev": [
            "pytest>=6.0.0",
            "pytest-benchmark>=3.4.0", 
            "black>=22.0.0",
            "ruff>=0.1.0",
            "mypy>=1.0.0",
        ],
    },
    
    license="MIT",
    zip_safe=False,
    include_package_data=True,
)