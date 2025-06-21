//! SIMD JSON Performance Benchmarks
//!
//! Benchmarks comparing SIMD vs standard JSON performance

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use fastest_core::utils::simd_json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestData {
    id: u64,
    name: String,
    tags: Vec<String>,
    metadata: HashMap<String, String>,
    nested: Option<Box<TestData>>,
}

impl TestData {
    fn generate(size: usize, depth: usize) -> Self {
        let tags: Vec<String> = (0..size)
            .map(|i| format!("tag_{}", i))
            .collect();
        
        let metadata: HashMap<String, String> = (0..size)
            .map(|i| (format!("key_{}", i), format!("value_{}", i)))
            .collect();
        
        let nested = if depth > 0 {
            Some(Box::new(Self::generate(size / 2, depth - 1)))
        } else {
            None
        };
        
        TestData {
            id: 12345,
            name: format!("test_data_{}", size),
            tags,
            metadata,
            nested,
        }
    }
}

fn benchmark_simd_vs_serde_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_small");
    
    let data = TestData::generate(10, 2);
    let json = serde_json::to_string(&data).unwrap();
    group.throughput(Throughput::Bytes(json.len() as u64));
    
    group.bench_function("simd_parse", |b| {
        b.iter(|| {
            let result: TestData = simd_json::from_str(&json).unwrap();
            black_box(result)
        });
    });
    
    group.bench_function("serde_parse", |b| {
        b.iter(|| {
            let result: TestData = serde_json::from_str(&json).unwrap();
            black_box(result)
        });
    });
    
    group.bench_function("simd_stringify", |b| {
        b.iter(|| {
            let json = simd_json::to_string(&data).unwrap();
            black_box(json)
        });
    });
    
    group.bench_function("serde_stringify", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&data).unwrap();
            black_box(json)
        });
    });
    
    group.finish();
}

fn benchmark_simd_vs_serde_large(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_large");
    
    let data = TestData::generate(1000, 5);
    let json = serde_json::to_string(&data).unwrap();
    group.throughput(Throughput::Bytes(json.len() as u64));
    
    group.bench_function("simd_parse", |b| {
        b.iter(|| {
            let result: TestData = simd_json::from_str(&json).unwrap();
            black_box(result)
        });
    });
    
    group.bench_function("serde_parse", |b| {
        b.iter(|| {
            let result: TestData = serde_json::from_str(&json).unwrap();
            black_box(result)
        });
    });
    
    group.finish();
}

fn benchmark_simd_test_items(c: &mut Criterion) {
    use fastest_core::test::discovery::TestItem;
    
    let mut group = c.benchmark_group("json_test_items");
    
    for count in [100, 1000, 10000].iter() {
        let items: Vec<TestItem> = (0..*count)
            .map(|i| TestItem {
                id: format!("test_file.py::test_func_{}", i),
                path: std::path::PathBuf::from("test_file.py"),
                function_name: format!("test_func_{}", i),
                line_number: Some((i + 1) as u32),
                decorators: smallvec::smallvec!["@pytest.mark.slow".to_string()],
                is_async: false,
                fixture_deps: smallvec::smallvec!["fixture1".to_string(), "fixture2".to_string()],
                class_name: if i % 10 == 0 { Some(format!("TestClass{}", i / 10)) } else { None },
                is_xfail: false,
                name: format!("test_func_{}", i),
                indirect_params: std::collections::HashMap::new(),
            })
            .collect();
        
        let json = serde_json::to_string(&items).unwrap();
        
        group.throughput(Throughput::Bytes(json.len() as u64));
        
        group.bench_with_input(
            BenchmarkId::new("simd_parse", count),
            &json,
            |b, json| {
                b.iter(|| {
                    let result: Vec<TestItem> = simd_json::from_str(json).unwrap();
                    black_box(result)
                });
            }
        );
        
        group.bench_with_input(
            BenchmarkId::new("serde_parse", count),
            &json,
            |b, json| {
                b.iter(|| {
                    let result: Vec<TestItem> = serde_json::from_str(json).unwrap();
                    black_box(result)
                });
            }
        );
    }
    
    group.finish();
}

fn benchmark_simd_pretty_print(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_pretty");
    
    let data = TestData::generate(100, 3);
    
    group.bench_function("simd_pretty", |b| {
        b.iter(|| {
            let json = simd_json::to_string_pretty(&data).unwrap();
            black_box(json)
        });
    });
    
    group.bench_function("serde_pretty", |b| {
        b.iter(|| {
            let json = serde_json::to_string_pretty(&data).unwrap();
            black_box(json)
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_simd_vs_serde_small,
    benchmark_simd_vs_serde_large,
    benchmark_simd_test_items,
    benchmark_simd_pretty_print
);
criterion_main!(benches);