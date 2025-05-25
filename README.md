# fastest

`fastest` is a minimal rust-powered Python testing framework. This repository
contains the initial skeleton of the tool using [pyo3](https://pyo3.rs/) and
[maturin](https://github.com/PyO3/maturin) to build a Python extension module
that exposes a simple CLI.

The project is inspired by tools like `uv` and `ruff`, aiming to eventually
provide blazing fast test execution for Python projects.

## Building

To build the Python package, install `maturin` and run:

```bash
maturin develop
```

This will compile the Rust extension and install the `fastest` package into
your current environment.

## Usage

```bash
fastest
```

Currently the implementation is a stub that prints a message.
