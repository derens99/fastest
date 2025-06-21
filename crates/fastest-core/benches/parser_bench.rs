//! Parser Performance Benchmarks
//!
//! Benchmarks for Python parsing performance

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use fastest_core::test::parser::Parser;
use std::path::Path;
use tempfile::TempDir;
use std::fs;

/// Generate Python code with specified characteristics
fn generate_python_code(
    functions: usize,
    classes: usize,
    methods_per_class: usize,
    fixtures: usize,
    decorators_per_item: usize,
) -> String {
    let mut code = String::from("import pytest\n\n");
    
    // Add fixtures
    for i in 0..fixtures {
        code.push_str(&format!(
            "@pytest.fixture(scope='function')\ndef fixture_{}():\n    return {}\n\n",
            i, i
        ));
    }
    
    // Add test functions
    for i in 0..functions {
        // Add decorators
        for j in 0..decorators_per_item {
            code.push_str(&format!("@pytest.mark.mark_{}\n", j));
        }
        code.push_str(&format!("def test_function_{}():\n    pass\n\n", i));
    }
    
    // Add test classes
    for i in 0..classes {
        code.push_str(&format!("class TestClass{}:\n", i));
        
        // Add setup/teardown
        code.push_str("    def setup_method(self):\n        pass\n\n");
        
        // Add test methods
        for j in 0..methods_per_class {
            for k in 0..decorators_per_item {
                code.push_str(&format!("    @pytest.mark.mark_{}\n", k));
            }
            code.push_str(&format!("    def test_method_{}(self):\n        pass\n\n", j));
        }
        
        code.push_str("    def teardown_method(self):\n        pass\n\n");
    }
    
    code
}

fn benchmark_parser_small_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_small");
    
    for size in [10, 50, 100].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let code = generate_python_code(size, 0, 0, 5, 1);
            
            b.iter(|| {
                let mut parser = Parser::new().unwrap();
                let _ = black_box(parser.parse_content(&code));
            });
        });
    }
    
    group.finish();
}

fn benchmark_parser_large_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_large");
    group.sample_size(20);
    
    for size in [500, 1000, 2000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let code = generate_python_code(size / 2, size / 4, 4, 20, 2);
            
            b.iter(|| {
                let mut parser = Parser::new().unwrap();
                let _ = black_box(parser.parse_content(&code));
            });
        });
    }
    
    group.finish();
}

fn benchmark_parser_complex_decorators(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_decorators");
    
    group.bench_function("100_tests_5_decorators", |b| {
        let mut code = String::from("import pytest\n\n");
        
        for i in 0..100 {
            code.push_str("@pytest.mark.slow\n");
            code.push_str("@pytest.mark.integration\n");
            code.push_str(&format!("@pytest.mark.parametrize('x', [1, 2, 3])\n"));
            code.push_str(&format!("@pytest.mark.skipif(True, reason='test')\n"));
            code.push_str(&format!("@pytest.mark.xfail\n"));
            code.push_str(&format!("def test_complex_{}(x):\n    pass\n\n", i));
        }
        
        b.iter(|| {
            let mut parser = Parser::new().unwrap();
            let _ = black_box(parser.parse_content(&code));
        });
    });
    
    group.finish();
}

fn benchmark_parser_class_heavy(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_classes");
    
    group.bench_function("50_classes_20_methods", |b| {
        let code = generate_python_code(0, 50, 20, 10, 1);
        
        b.iter(|| {
            let mut parser = Parser::new().unwrap();
            let _ = black_box(parser.parse_content(&code));
        });
    });
    
    group.finish();
}

fn benchmark_parser_fixtures(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_fixtures");
    
    group.bench_function("100_fixtures_varied_scopes", |b| {
        let mut code = String::from("import pytest\n\n");
        
        let scopes = ["function", "class", "module", "session"];
        
        for i in 0..100 {
            let scope = scopes[i % scopes.len()];
            code.push_str(&format!(
                "@pytest.fixture(scope='{}', autouse={})\ndef fixture_{}():\n    yield {}\n    # teardown\n\n",
                scope,
                i % 3 == 0,
                i,
                i
            ));
        }
        
        // Add some tests that use fixtures
        for i in 0..50 {
            code.push_str(&format!(
                "def test_with_fixtures(fixture_{}, fixture_{}):\n    pass\n\n",
                i * 2,
                i * 2 + 1
            ));
        }
        
        b.iter(|| {
            let mut parser = Parser::new().unwrap();
            let _ = black_box(parser.parse_content(&code));
        });
    });
    
    group.finish();
}

fn benchmark_parser_async_tests(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_async");
    
    group.bench_function("100_async_tests", |b| {
        let mut code = String::from("import pytest\nimport asyncio\n\n");
        
        for i in 0..100 {
            code.push_str(&format!(
                "async def test_async_{}():\n    await asyncio.sleep(0)\n\n",
                i
            ));
        }
        
        b.iter(|| {
            let mut parser = Parser::new().unwrap();
            let _ = black_box(parser.parse_content(&code));
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_parser_small_files,
    benchmark_parser_large_files,
    benchmark_parser_complex_decorators,
    benchmark_parser_class_heavy,
    benchmark_parser_fixtures,
    benchmark_parser_async_tests
);
criterion_main!(benches);