use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

/// Executes fixture code and returns the fixture values
pub struct FixtureExecutor {
    fixture_code: HashMap<String, String>,
}

impl FixtureExecutor {
    pub fn new() -> Self {
        Self {
            fixture_code: HashMap::new(),
        }
    }

    /// Register fixture implementation code
    pub fn register_fixture_code(&mut self, fixture_name: String, code: String) {
        self.fixture_code.insert(fixture_name, code);
    }

    /// Execute fixtures and return their values
    pub fn execute_fixtures(
        &self,
        fixtures: &[String],
        test_path: &Path,
        fixture_values: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>> {
        let mut results = HashMap::new();

        // Build Python code to execute fixtures
        let python_code = self.build_fixture_execution_code(fixtures, test_path, fixture_values)?;

        // Execute Python code and parse results
        let output = std::process::Command::new("python")
            .arg("-c")
            .arg(&python_code)
            .output()
            .map_err(|e| anyhow!("Failed to execute fixtures: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Fixture execution failed: {}", stderr));
        }

        // Parse JSON output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let json_results: HashMap<String, Value> = serde_json::from_str(&stdout)
            .map_err(|e| anyhow!("Failed to parse fixture results: {}", e))?;

        results.extend(json_results);
        Ok(results)
    }

    fn build_fixture_execution_code(
        &self,
        fixtures: &[String],
        test_path: &Path,
        existing_values: &HashMap<String, Value>,
    ) -> Result<String> {
        let test_dir = test_path
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());

        let module_name = test_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "test".to_string());

        // Build the Python code
        let mut code = format!(
            r#"
import sys
import json
import traceback

# Add test directory to path
sys.path.insert(0, r'{}')

# Import the test module
try:
    import {} as test_module
except ImportError as e:
    print(json.dumps({{"error": f"Failed to import module: {{e}}"}}))
    sys.exit(1)

# Fixture results
fixture_results = {{}}

# Existing fixture values
existing_fixtures = {}

"#,
            test_dir,
            module_name,
            serde_json::to_string(existing_values)?
        );

        // Add code to execute each fixture
        for fixture_name in fixtures {
            code.push_str(&format!(
                r#"
# Execute fixture: {}
try:
    if hasattr(test_module, '{}'):
        fixture_func = getattr(test_module, '{}')
        # Get fixture dependencies from function signature
        import inspect
        sig = inspect.signature(fixture_func)
        kwargs = {{}}
        for param_name in sig.parameters:
            if param_name in existing_fixtures:
                kwargs[param_name] = existing_fixtures[param_name]
            elif param_name in fixture_results:
                kwargs[param_name] = fixture_results[param_name]
        
        # Call fixture
        result = fixture_func(**kwargs)
        fixture_results['{}'] = result
        
        # Handle generator fixtures (yield)
        if inspect.isgeneratorfunction(fixture_func):
            # For generator fixtures, we only get the yielded value
            try:
                fixture_results['{}'] = next(result)
            except StopIteration as e:
                if hasattr(e, 'value'):
                    fixture_results['{}'] = e.value
except Exception as e:
    print(json.dumps({{"error": f"Failed to execute fixture {}: {{e}}"}}))
    traceback.print_exc()
    sys.exit(1)
"#,
                fixture_name,
                fixture_name,
                fixture_name,
                fixture_name,
                fixture_name,
                fixture_name,
                fixture_name
            ));
        }

        // Output results as JSON
        code.push_str(
            "\n# Output fixture results as JSON\nprint(json.dumps(fixture_results, default=str))",
        );

        Ok(code)
    }
}

impl Default for FixtureExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate Python code that includes fixture injection
pub fn generate_test_code_with_fixtures(
    test: &crate::discovery::TestItem,
    fixture_values: &HashMap<String, Value>,
) -> String {
    let test_dir = test
        .path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());

    let module_name = test
        .path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "test".to_string());

    // Build fixture kwargs string
    let _fixture_kwargs = if test.fixture_deps.is_empty() {
        String::new()
    } else {
        let mut kwargs = Vec::new();
        for fixture_name in &test.fixture_deps {
            if let Some(value) = fixture_values.get(fixture_name) {
                kwargs.push(format!(
                    "{}={}",
                    fixture_name,
                    serde_json::to_string(value).unwrap_or_else(|_| "None".to_string())
                ));
            }
        }
        if kwargs.is_empty() {
            String::new()
        } else {
            format!("({})", kwargs.join(", "))
        }
    };

    if test.is_async {
        format!(
            r#"
import sys
import os
import asyncio
import traceback
import json

sys.path.insert(0, r'{}')

try:
    import {} as test_module
    {}
    
    # Fixture values
    fixture_values = {}
    
    async def run_test():
        try:
            # Prepare fixture arguments
            kwargs = {{}}
            for fixture_name in {}:
                if fixture_name in fixture_values:
                    kwargs[fixture_name] = fixture_values[fixture_name]
            
            result = await {}
            print("Test passed")
        except Exception as e:
            print(f"Test failed: {{e}}", file=sys.stderr)
            traceback.print_exc(file=sys.stderr)
            sys.exit(1)
    
    asyncio.run(run_test())
except Exception as e:
    print(f"Failed to import or run test: {{e}}", file=sys.stderr)
    traceback.print_exc(file=sys.stderr)
    sys.exit(1)
"#,
            test_dir,
            module_name,
            if let Some(class_name) = &test.class_name {
                format!("\n    test_class = getattr(test_module, '{}')\n    test_instance = test_class()", class_name)
            } else {
                format!(
                    "\n    test_func = getattr(test_module, '{}')",
                    test.function_name
                )
            },
            serde_json::to_string(fixture_values).unwrap_or_else(|_| "{}".to_string()),
            serde_json::to_string(&test.fixture_deps).unwrap_or_else(|_| "[]".to_string()),
            if test.class_name.is_some() {
                format!("test_instance.{}(**kwargs)", test.function_name)
            } else {
                "test_func(**kwargs)".to_string()
            }
        )
    } else {
        format!(
            r#"
import sys
import os
import traceback
import json

sys.path.insert(0, r'{}')

try:
    import {} as test_module
    {}
    
    # Fixture values
    fixture_values = {}
    
    # Prepare fixture arguments
    kwargs = {{}}
    for fixture_name in {}:
        if fixture_name in fixture_values:
            kwargs[fixture_name] = fixture_values[fixture_name]
    
    # Run the test
    {}
    print("Test passed")
except Exception as e:
    print(f"Test failed: {{e}}", file=sys.stderr)
    traceback.print_exc(file=sys.stderr)
    sys.exit(1)
"#,
            test_dir,
            module_name,
            if let Some(class_name) = &test.class_name {
                format!("\n    test_class = getattr(test_module, '{}')\n    test_instance = test_class()", class_name)
            } else {
                format!(
                    "\n    test_func = getattr(test_module, '{}')",
                    test.function_name
                )
            },
            serde_json::to_string(fixture_values).unwrap_or_else(|_| "{}".to_string()),
            serde_json::to_string(&test.fixture_deps).unwrap_or_else(|_| "[]".to_string()),
            if test.class_name.is_some() {
                format!("test_instance.{}(**kwargs)", test.function_name)
            } else {
                "test_func(**kwargs)".to_string()
            }
        )
    }
}
