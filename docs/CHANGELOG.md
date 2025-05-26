### Added
- **Parametrized Test Support** 🎭
  - Full support for `@pytest.mark.parametrize` decorator
  - Full support for `@fastest.mark.parametrize` decorator (native syntax)
  - Handles single and multiple parameters
  - Supports multiple decorators (cartesian product)
  - Complex parameter values (tuples, lists, strings, numbers)
  - Proper test ID generation with parameter values
  - Works with both regex and AST parsers 