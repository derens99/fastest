# Invalid Syntax Fixtures

This directory preserves source files that are useful for parser and error-handling development but should not be collected during ordinary pytest compatibility runs.

Files under `fixtures/` use a `.fixture` suffix on purpose. Some contain invalid Python identifiers, such as emoji in function names, so renaming them to `.py` would make broad suite runs fail during collection.

Use these fixtures only from targeted tests or manual parser experiments.
