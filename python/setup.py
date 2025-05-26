import os
import sys
from setuptools import setup, find_packages
from pathlib import Path

# Read the README
this_directory = Path(__file__).parent.parent
long_description = (this_directory / "README.md").read_text()

# Read version
version_file = Path(__file__).parent / "fastest_runner" / "__init__.py"
version = None
with open(version_file) as f:
    for line in f:
        if line.startswith("__version__"):
            version = line.split("=")[1].strip().strip('"').strip("'")
            break

if not version:
    raise RuntimeError("Cannot find version information")

setup(
    name="fastest-runner",
    version=version,
    author="Your Name",
    author_email="your.email@example.com",
    description="A blazing fast Python test runner built with Rust",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/yourusername/fastest",
    packages=find_packages(),
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
        "Operating System :: OS Independent",
    ],
    python_requires=">=3.8",
    entry_points={
        "console_scripts": [
            "fastest=fastest_runner:main",
        ],
    },
    include_package_data=True,
    package_data={
        "fastest_runner": ["bin/*"],
    },
    zip_safe=False,
) 