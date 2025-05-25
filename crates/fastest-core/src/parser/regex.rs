use std::path::Path;
use crate::error::Result;

#[derive(Debug, Clone)]
pub struct TestFunction {
    pub name: String,
    pub line_number: usize,
    pub is_async: bool,
    pub class_name: Option<String>,
    pub decorators: Vec<String>,
}

pub fn parse_test_file(path: &Path) -> Result<Vec<TestFunction>> {
    let content = std::fs::read_to_string(path)?;
    let mut tests = Vec::new();
    let mut current_class: Option<String> = None;
    let mut class_indent = 0;
    
    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        
        // Class definition
        if trimmed.starts_with("class ") && trimmed.ends_with(':') {
            if let Some(class_name) = extract_class_name(trimmed) {
                current_class = Some(class_name);
                class_indent = indent;
            }
        }
        
        // Check if we've left the class
        if current_class.is_some() && !line.trim().is_empty() {
            // If we see a non-method definition at class level or less, we've left the class
            if indent <= class_indent && !trimmed.starts_with("def ") && !trimmed.starts_with("async def ") {
                current_class = None;
            }
            // Also check if this is a function definition at wrong indentation (module level)
            else if (trimmed.starts_with("def ") || trimmed.starts_with("async def ")) && indent <= class_indent {
                current_class = None;
            }
        }
        
        // Function definition
        if trimmed.starts_with("def test_") || trimmed.starts_with("async def test_") {
            if let Some(func_name) = extract_function_name(trimmed) {
                tests.push(TestFunction {
                    name: func_name,
                    line_number: line_num + 1,
                    is_async: trimmed.starts_with("async "),
                    class_name: current_class.clone(),
                    decorators: Vec::new(), // Simple parser doesn't track decorators
                });
            }
        }
    }
    
    Ok(tests)
}

fn extract_class_name(line: &str) -> Option<String> {
    let class_start = "class ".len();
    let class_part = &line[class_start..];
    if let Some(paren_pos) = class_part.find('(') {
        Some(class_part[..paren_pos].trim().to_string())
    } else if let Some(colon_pos) = class_part.find(':') {
        Some(class_part[..colon_pos].trim().to_string())
    } else {
        None
    }
}

fn extract_function_name(line: &str) -> Option<String> {
    let def_pos = if line.starts_with("async ") {
        line.find("def ")? + 4
    } else {
        4 // "def ".len()
    };
    
    let func_part = &line[def_pos..];
    if let Some(paren_pos) = func_part.find('(') {
        Some(func_part[..paren_pos].trim().to_string())
    } else {
        None
    }
}