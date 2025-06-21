//! Cache Performance Benchmarks
//!
//! Benchmarks for discovery cache performance

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use fastest_core::cache::DiscoveryCache;
use fastest_core::test::discovery::TestItem;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create test items for benchmarking
fn create_test_items(count: usize) -> Vec<TestItem> {
    (0..count)
        .map(|i| TestItem {
            id: format!("test_file.py::test_func_{}", i),
            path: PathBuf::from("test_file.py"),
            function_name: format!("test_func_{}", i),
            line_number: Some((i + 1) as u32),
            decorators: smallvec::smallvec![],
            is_async: false,
            fixture_deps: smallvec::smallvec!["fixture1".to_string(), "fixture2".to_string()],
            class_name: None,
            is_xfail: false,
            name: format!("test_func_{}", i),
            indirect_params: std::collections::HashMap::new(),
        })
        .collect()
}

fn benchmark_cache_save(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_save");
    
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let items = create_test_items(size);
            
            b.iter_batched(
                || {
                    let dir = TempDir::new().unwrap();
                    let cache_path = dir.path().join(".fastest_cache");
                    let cache = DiscoveryCache::new(cache_path);
                    (cache, items.clone(), dir)
                },
                |(cache, items, _dir)| {
                    for (i, chunk) in items.chunks(100).enumerate() {
                        let path = PathBuf::from(format!("test_file_{}.py", i));
                        cache.save_tests(&path, chunk, &format!("hash_{}", i)).unwrap();
                    }
                },
                criterion::BatchSize::SmallInput
            );
        });
    }
    
    group.finish();
}

fn benchmark_cache_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_load");
    
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    // Setup: create cache with data
                    let dir = TempDir::new().unwrap();
                    let cache_path = dir.path().join(".fastest_cache");
                    let cache = DiscoveryCache::new(cache_path);
                    let items = create_test_items(size);
                    
                    // Save items to cache
                    for (i, chunk) in items.chunks(100).enumerate() {
                        let path = PathBuf::from(format!("test_file_{}.py", i));
                        cache.save_tests(&path, chunk, &format!("hash_{}", i)).unwrap();
                    }
                    
                    (cache, dir)
                },
                |(cache, _dir)| {
                    // Benchmark: load all items from cache
                    let files_count = (size + 99) / 100;
                    for i in 0..files_count {
                        let path = PathBuf::from(format!("test_file_{}.py", i));
                        let _ = cache.load_tests(&path, &format!("hash_{}", i));
                    }
                },
                criterion::BatchSize::SmallInput
            );
        });
    }
    
    group.finish();
}

fn benchmark_cache_has_valid_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_validation");
    
    group.bench_function("check_1000_files", |b| {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join(".fastest_cache");
        let cache = DiscoveryCache::new(cache_path);
        let items = create_test_items(100);
        
        // Pre-populate cache
        for i in 0..1000 {
            let path = PathBuf::from(format!("test_file_{}.py", i));
            cache.save_tests(&path, &items, &format!("hash_{}", i)).unwrap();
        }
        
        b.iter(|| {
            for i in 0..1000 {
                let path = PathBuf::from(format!("test_file_{}.py", i));
                let _ = black_box(cache.has_valid_cache(&path, &format!("hash_{}", i)));
            }
        });
    });
    
    group.finish();
}

fn benchmark_cache_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_compression");
    
    group.bench_function("compress_10000_items", |b| {
        let items = create_test_items(10000);
        
        b.iter_batched(
            || {
                let dir = TempDir::new().unwrap();
                let cache_path = dir.path().join(".fastest_cache");
                let cache = DiscoveryCache::new(cache_path);
                (cache, items.clone(), dir)
            },
            |(cache, items, _dir)| {
                // Save with compression
                let path = PathBuf::from("large_test_file.py");
                cache.save_tests(&path, &items, "hash_large").unwrap();
            },
            criterion::BatchSize::SmallInput
        );
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_cache_save,
    benchmark_cache_load,
    benchmark_cache_has_valid_cache,
    benchmark_cache_compression
);
criterion_main!(benches);