//! Discovery Performance Benchmarks
//!
//! Benchmarks for test discovery performance

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use fastest_core::test::discovery::{discover_tests, discover_tests_with_filtering};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a test file with specified number of tests
fn create_test_file(dir: &TempDir, name: &str, num_tests: usize) -> PathBuf {
    let content = (0..num_tests)
        .map(|i| format!("def test_func_{}():\n    pass\n", i))
        .collect::<String>();
    
    let path = dir.path().join(name);
    fs::write(&path, content).unwrap();
    path
}

/// Create test files with classes
fn create_class_test_file(dir: &TempDir, name: &str, num_classes: usize, tests_per_class: usize) -> PathBuf {
    let mut content = String::new();
    
    for i in 0..num_classes {
        content.push_str(&format!("class TestClass{}:\n", i));
        for j in 0..tests_per_class {
            content.push_str(&format!("    def test_method_{}(self):\n        pass\n", j));
        }
        content.push('\n');
    }
    
    let path = dir.path().join(name);
    fs::write(&path, content).unwrap();
    path
}

/// Create parametrized test file
fn create_parametrized_test_file(dir: &TempDir, name: &str, num_tests: usize, params_per_test: usize) -> PathBuf {
    let mut content = String::from("import pytest\n\n");
    
    for i in 0..num_tests {
        let params = (0..params_per_test)
            .map(|j| format!("{}", j))
            .collect::<Vec<_>>()
            .join(", ");
        
        content.push_str(&format!(
            "@pytest.mark.parametrize('x', [{}])\ndef test_param_{}(x):\n    pass\n\n",
            params, i
        ));
    }
    
    let path = dir.path().join(name);
    fs::write(&path, content).unwrap();
    path
}

fn benchmark_discovery_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("discovery_small");
    
    for size in [10, 50, 100].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    let dir = TempDir::new().unwrap();
                    create_test_file(&dir, "test_suite.py", size);
                    dir
                },
                |dir| {
                    let _ = discover_tests(&[dir.path().to_path_buf()]);
                },
                criterion::BatchSize::SmallInput
            );
        });
    }
    
    group.finish();
}

fn benchmark_discovery_large(c: &mut Criterion) {
    let mut group = c.benchmark_group("discovery_large");
    group.sample_size(20);
    
    for size in [500, 1000, 5000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    let dir = TempDir::new().unwrap();
                    // Create multiple files
                    let files_count = size / 100;
                    let tests_per_file = size / files_count;
                    
                    for i in 0..files_count {
                        create_test_file(&dir, &format!("test_suite_{}.py", i), tests_per_file);
                    }
                    dir
                },
                |dir| {
                    let _ = discover_tests(&[dir.path().to_path_buf()]);
                },
                criterion::BatchSize::LargeInput
            );
        });
    }
    
    group.finish();
}

fn benchmark_discovery_classes(c: &mut Criterion) {
    let mut group = c.benchmark_group("discovery_classes");
    
    group.bench_function("100_classes_10_methods", |b| {
        b.iter_batched(
            || {
                let dir = TempDir::new().unwrap();
                create_class_test_file(&dir, "test_classes.py", 100, 10);
                dir
            },
            |dir| {
                let _ = discover_tests(&[dir.path().to_path_buf()]);
            },
            criterion::BatchSize::SmallInput
        );
    });
    
    group.finish();
}

fn benchmark_discovery_parametrized(c: &mut Criterion) {
    let mut group = c.benchmark_group("discovery_parametrized");
    
    group.bench_function("50_tests_20_params", |b| {
        b.iter_batched(
            || {
                let dir = TempDir::new().unwrap();
                create_parametrized_test_file(&dir, "test_params.py", 50, 20);
                dir
            },
            |dir| {
                let _ = discover_tests(&[dir.path().to_path_buf()]);
            },
            criterion::BatchSize::SmallInput
        );
    });
    
    group.finish();
}

fn benchmark_discovery_with_filtering(c: &mut Criterion) {
    let mut group = c.benchmark_group("discovery_filtering");
    
    group.bench_function("1000_tests_filtered", |b| {
        b.iter_batched(
            || {
                let dir = TempDir::new().unwrap();
                create_test_file(&dir, "test_large.py", 1000);
                dir
            },
            |dir| {
                let _ = discover_tests_with_filtering(&[dir.path().to_path_buf()], true);
            },
            criterion::BatchSize::SmallInput
        );
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_discovery_small,
    benchmark_discovery_large,
    benchmark_discovery_classes,
    benchmark_discovery_parametrized,
    benchmark_discovery_with_filtering
);
criterion_main!(benches);