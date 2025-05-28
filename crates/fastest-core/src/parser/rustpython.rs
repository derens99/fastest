use crate::error::{Error, Result};
use crate::parser::{FixtureDefinition, TestFunction};
use rustpython_parser::{ast, Parse};
use std::path::Path;

pub struct RustPythonParser;

impl RustPythonParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse_file(
        &self,
        path: &Path,
    ) -> Result<(Vec<FixtureDefinition>, Vec<TestFunction>)> {
        let content = std::fs::read_to_string(path)?;
        self.parse_content(&content)
    }

    pub fn parse_content(
        &self,
        content: &str,
    ) -> Result<(Vec<FixtureDefinition>, Vec<TestFunction>)> {
        let module = ast::Suite::parse(content, "<test>")
            .map_err(|e| Error::Parse(format!("Failed to parse Python: {:?}", e)))?;

        let mut fixtures = Vec::new();
        let mut tests = Vec::new();

        for stmt in module {
            self.visit_stmt(&stmt, None, &mut fixtures, &mut tests);
        }

        Ok((fixtures, tests))
    }

    fn visit_stmt(
        &self,
        stmt: &ast::Stmt,
        class_name: Option<&str>,
        fixtures: &mut Vec<FixtureDefinition>,
        tests: &mut Vec<TestFunction>,
    ) {
        match stmt {
            ast::Stmt::FunctionDef(func_def) => {
                self.handle_function(func_def, class_name, fixtures, tests);
            }
            ast::Stmt::AsyncFunctionDef(async_func_def) => {
                self.handle_async_function(async_func_def, class_name, fixtures, tests);
            }
            ast::Stmt::ClassDef(class_def) => {
                // Visit methods inside the class
                for stmt in &class_def.body {
                    self.visit_stmt(stmt, Some(&class_def.name.to_string()), fixtures, tests);
                }
            }
            _ => {}
        }
    }

    fn handle_function(
        &self,
        func_def: &ast::StmtFunctionDef,
        class_name: Option<&str>,
        fixtures: &mut Vec<FixtureDefinition>,
        tests: &mut Vec<TestFunction>,
    ) {
        let decorators = self.extract_decorators_from_stmt(func_def);
        
        // Check if it's a fixture
        if decorators.iter().any(|d| d.contains("fixture")) {
            let fixture = self.parse_fixture(func_def, &decorators);
            if let Some(fixture) = fixture {
                fixtures.push(fixture);
            }
        }
        
        // Check if it's a test
        if func_def.name.to_string().starts_with("test_") {
            let test = TestFunction {
                name: func_def.name.to_string(),
                line_number: 0,
                is_async: false,
                class_name: class_name.map(String::from),
                decorators,
                parameters: self.extract_parameters(&func_def.args),
            };
            tests.push(test);
        }
    }

    fn handle_async_function(
        &self,
        async_func_def: &ast::StmtAsyncFunctionDef,
        class_name: Option<&str>,
        fixtures: &mut Vec<FixtureDefinition>,
        tests: &mut Vec<TestFunction>,
    ) {
        let decorators = self.extract_decorators_from_async_stmt(async_func_def);
        
        // Check if it's a test
        if async_func_def.name.to_string().starts_with("test_") {
            let test = TestFunction {
                name: async_func_def.name.to_string(),
                line_number: 0,
                is_async: true,
                class_name: class_name.map(String::from),
                decorators,
                parameters: self.extract_parameters(&async_func_def.args),
            };
            tests.push(test);
        }
    }

    fn extract_decorators_from_stmt(&self, func_def: &ast::StmtFunctionDef) -> Vec<String> {
        func_def.decorator_list
            .iter()
            .map(|decorator| self.expr_to_string(decorator))
            .collect()
    }

    fn extract_decorators_from_async_stmt(&self, async_func_def: &ast::StmtAsyncFunctionDef) -> Vec<String> {
        async_func_def.decorator_list
            .iter()
            .map(|decorator| self.expr_to_string(decorator))
            .collect()
    }

    fn expr_to_string(&self, expr: &ast::Expr) -> String {
        match expr {
            ast::Expr::Name(name) => name.id.to_string(),
            ast::Expr::Attribute(attr) => {
                format!("{}.{}", self.expr_to_string(&attr.value), attr.attr)
            }
            ast::Expr::Call(call) => {
                let func_name = self.expr_to_string(&call.func);
                let args = call
                    .args
                    .iter()
                    .map(|arg| self.expr_to_string(arg))
                    .collect::<Vec<_>>()
                    .join(", ");
                
                let kwargs = call
                    .keywords
                    .iter()
                    .map(|kw| {
                        if let Some(arg) = &kw.arg {
                            format!("{}={}", arg, self.expr_to_string(&kw.value))
                        } else {
                            self.expr_to_string(&kw.value)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                
                let all_args = if args.is_empty() {
                    kwargs
                } else if kwargs.is_empty() {
                    args
                } else {
                    format!("{}, {}", args, kwargs)
                };
                
                format!("{}({})", func_name, all_args)
            }
            ast::Expr::Constant(constant) => {
                match &constant.value {
                    ast::Constant::Str(s) => format!("\"{}\"", s),
                    ast::Constant::Int(i) => i.to_string(),
                    ast::Constant::Float(f) => f.to_string(),
                    ast::Constant::Bool(b) => b.to_string(),
                    ast::Constant::None => "None".to_string(),
                    _ => "?".to_string(),
                }
            }
            ast::Expr::List(list) => {
                let items = list
                    .elts
                    .iter()
                    .map(|e| self.expr_to_string(e))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{}]", items)
            }
            ast::Expr::Tuple(tuple) => {
                let items = tuple
                    .elts
                    .iter()
                    .map(|e| self.expr_to_string(e))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({})", items)
            }
            ast::Expr::Dict(dict) => {
                let items = dict.keys.iter().zip(&dict.values)
                    .map(|(k, v)| {
                        if let Some(key) = k {
                            format!("{}: {}", self.expr_to_string(key), self.expr_to_string(v))
                        } else {
                            format!("**{}", self.expr_to_string(v))
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{}}}", items)
            }
            _ => "...".to_string(),
        }
    }

    fn extract_parameters(&self, args: &ast::Arguments) -> Vec<String> {
        let mut params = Vec::new();
        
        // Regular args
        for arg in &args.args {
            params.push(arg.def.arg.to_string());
        }
        
        // *args
        if let Some(vararg) = &args.vararg {
            params.push(format!("*{}", vararg.arg));
        }
        
        // **kwargs
        if let Some(kwarg) = &args.kwarg {
            params.push(format!("**{}", kwarg.arg));
        }
        
        params
    }

    fn parse_fixture(
        &self,
        func_def: &ast::StmtFunctionDef,
        decorators: &[String],
    ) -> Option<FixtureDefinition> {
        let mut scope = "function".to_string();
        let mut autouse = false;

        // Parse fixture decorator for scope and autouse
        for decorator in decorators {
            if decorator.contains("fixture") {
                if decorator.contains("scope=\"session\"") || decorator.contains("scope='session'") {
                    scope = "session".to_string();
                } else if decorator.contains("scope=\"module\"") || decorator.contains("scope='module'") {
                    scope = "module".to_string();
                } else if decorator.contains("scope=\"class\"") || decorator.contains("scope='class'") {
                    scope = "class".to_string();
                }
                
                if decorator.contains("autouse=True") {
                    autouse = true;
                }
            }
        }

        Some(FixtureDefinition {
            name: func_def.name.to_string(),
            scope,
            autouse,
            line_number: 0,
            is_async: false,
            params: Vec::new(),
            decorators: decorators.to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_test() {
        let parser = RustPythonParser::new();
        let content = r#"
def test_simple():
    assert True
"#;
        let (fixtures, tests) = parser.parse_content(content).unwrap();
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].name, "test_simple");
        assert!(!tests[0].is_async);
    }

    #[test]
    fn test_parse_parametrized_test() {
        let parser = RustPythonParser::new();
        let content = r#"
@pytest.mark.parametrize("x,y", [(1, 2), (3, 4)])
def test_add(x, y):
    assert x + y > 0
"#;
        let (_, tests) = parser.parse_content(content).unwrap();
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].name, "test_add");
        assert_eq!(tests[0].decorators.len(), 1);
        assert!(tests[0].decorators[0].contains("parametrize"));
    }

    #[test]
    fn test_parse_fixture() {
        let parser = RustPythonParser::new();
        let content = r#"
@pytest.fixture(scope="module", autouse=True)
def setup_module():
    return "setup"
"#;
        let (fixtures, _) = parser.parse_content(content).unwrap();
        assert_eq!(fixtures.len(), 1);
        assert_eq!(fixtures[0].name, "setup_module");
        assert_eq!(fixtures[0].scope, "module");
        assert!(fixtures[0].autouse);
    }
} 