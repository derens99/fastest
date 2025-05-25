# Fastest

A high-performance Python testing framework built with Rust.

## Features

- Fast test execution with Rust-powered core
- Simple and intuitive Python API
- Parallel test execution
- Detailed test reporting
- Easy integration with existing test suites

## Installation

### Prerequisites

- Python 3.7 or higher
- Rust toolchain (for development)

### From PyPI

```bash
pip install fastest
```

### From Source

1. Clone the repository:
```bash
git clone https://github.com/yourusername/fastest.git
cd fastest
```

2. Install the Python package:
```bash
cd python
pip install -e .
```

## Usage

Here's a simple example of how to use Fastest:

```python
from fastest import run_tests, ExampleTest

# Define your test
class MyTest(ExampleTest):
    def test_something(self):
        assert 1 + 1 == 2

# Run the tests
results = run_tests([MyTest()])
print(results)
```

## Development

### Setting up the development environment

1. Create a virtual environment:
```bash
python -m venv .venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate
```

2. Install development dependencies:
```bash
pip install -e ".[dev]"
```

3. Build the Rust extension:
```bash
cargo build
```

### Running tests

```bash
# Run Python tests
python -m pytest

# Run Rust tests
cargo test
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details. 