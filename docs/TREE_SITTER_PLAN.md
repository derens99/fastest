# Tree-sitter Python Parser Implementation Plan

## Overview
Replace the current regex-based parser with tree-sitter for more accurate and potentially faster Python test discovery.

## Current Limitations of Regex Parser
1. Can't handle complex decorators properly
2. Misses tests in nested classes
3. Fragile with unusual formatting
4. No understanding of Python syntax context
5. Difficult to extend for pytest features

## Implementation Steps

### Step 1: Add Dependencies
```toml
# crates/fastest-core/Cargo.toml
[dependencies]
tree-sitter = "0.22"
tree-sitter-python = "0.21"
once_cell = "1.19"  # For language singleton
```

### Step 2: Create AST Parser Module
```rust
// crates/fastest-core/src/parser/ast.rs
use tree_sitter::{Parser, Language, Node, Query, QueryCursor};
use tree_sitter_python;

pub struct AstParser {
    parser: Parser,
}

impl AstParser {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_python::language())?;
        Ok(Self { parser })
    }
    
    pub fn parse_file(&mut self, content: &str) -> Result<Vec<TestFunction>> {
        let tree = self.parser.parse(content, None)
            .ok_or_else(|| anyhow!("Failed to parse Python file"))?;
        
        let root = tree.root_node();
        let mut tests = Vec::new();
        
        self.visit_node(root, content, &mut tests)?;
        Ok(tests)
    }
    
    fn visit_node(&self, node: Node, source: &str, tests: &mut Vec<TestFunction>) -> Result<()> {
        match node.kind() {
            "function_definition" => {
                if let Some(name) = self.get_function_name(node, source) {
                    if name.starts_with("test_") {
                        tests.push(self.extract_test_function(node, source)?);
                    }
                }
            }
            "class_definition" => {
                // Check if class name starts with Test
                if let Some(name) = self.get_class_name(node, source) {
                    if name.starts_with("Test") {
                        // Visit methods
                        self.visit_class_methods(node, source, tests, &name)?;
                    }
                }
            }
            _ => {}
        }
        
        // Recursively visit children
        for child in node.children(&mut node.walk()) {
            self.visit_node(child, source, tests)?;
        }
        
        Ok(())
    }
}
```

### Step 3: Query-based Approach (Alternative)
```rust
// Using tree-sitter queries for more efficient parsing
const PYTHON_TEST_QUERY: &str = r#"
(function_definition
  name: (identifier) @function.name
  parameters: (parameters) @function.params
  (#match? @function.name "^test_")
) @test.function

(class_definition
  name: (identifier) @class.name
  (#match? @class.name "^Test")
  body: (block
    (function_definition
      name: (identifier) @method.name
      (#match? @method.name "^test_")
    ) @test.method
  )
) @test.class
"#;
```

### Step 4: Feature Extraction
```rust
impl AstParser {
    fn extract_test_function(&self, node: Node, source: &str) -> Result<TestFunction> {
        let name = self.get_function_name(node, source)?;
        let line_number = node.start_position().row + 1;
        let is_async = self.is_async_function(node);
        let decorators = self.get_decorators(node, source);
        
        Ok(TestFunction {
            name,
            line_number,
            is_async,
            decorators,
            // Future: parameters for parametrized tests
        })
    }
    
    fn get_decorators(&self, node: Node, source: &str) -> Vec<String> {
        // Find decorator nodes
        let mut decorators = Vec::new();
        if let Some(decorator_list) = node.child_by_field_name("decorator_list") {
            for decorator in decorator_list.children(&mut decorator_list.walk()) {
                if decorator.kind() == "decorator" {
                    let text = &source[decorator.byte_range()];
                    decorators.push(text.to_string());
                }
            }
        }
        decorators
    }
}
```

### Step 5: Benchmarking
```rust
#[cfg(test)]
mod benches {
    use super::*;
    use std::time::Instant;
    
    #[test]
    fn benchmark_parsers() {
        let content = std::fs::read_to_string("large_test_file.py").unwrap();
        
        // Regex parser
        let start = Instant::now();
        let regex_results = regex_parser::parse(&content);
        let regex_time = start.elapsed();
        
        // AST parser
        let start = Instant::now();
        let ast_results = AstParser::new().parse_file(&content);
        let ast_time = start.elapsed();
        
        println!("Regex: {:?}, found {} tests", regex_time, regex_results.len());
        println!("AST: {:?}, found {} tests", ast_time, ast_results.len());
    }
}
```

## Benefits of Tree-sitter

1. **Accurate Parsing**: Understands Python syntax fully
2. **Error Recovery**: Handles malformed code gracefully
3. **Incremental Parsing**: Can reparse only changed portions
4. **Query Language**: Powerful pattern matching
5. **Future Features**: Foundation for fixtures, parametrize, etc.

## Migration Strategy

1. Implement alongside regex parser
2. Add feature flag: `--parser=ast|regex`
3. Compare results on test corpus
4. Switch default when stable
5. Remove regex parser in future version

## Performance Considerations

- Tree-sitter parsing might be slower for simple files
- But more accurate and handles edge cases
- Can optimize with queries instead of visitor pattern
- Memory usage should be similar

## Testing Plan

1. Parse all test files in test_project
2. Compare with regex parser results
3. Add test cases for:
   - Decorated tests
   - Nested classes
   - Async tests
   - Parametrized tests
   - Unusual formatting

## Timeline

- Week 1: Basic implementation
- Week 2: Feature parity with regex parser
- Week 3: Optimization and benchmarking
- Week 4: Integration and migration 