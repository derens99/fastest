//! Cache Performance Benchmarks
//!
//! Benchmarks for discovery cache performance

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use fastest_core::cache::DiscoveryCache;
use fastest_core::test::discovery::TestItem;
use std::fs;
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

fn write_test_file(dir: &TempDir, index: usize) -> PathBuf {
    let path = dir.path().join(format!("test_file_{}.py", index));
    fs::write(
        &path,
        format!("def test_func_{}():\n    assert True\n", index),
    )
    .unwrap();
    path
}

fn benchmark_cache_save(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_save");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let items = create_test_items(size);

            b.iter_batched(
                || {
                    let dir = TempDir::new().unwrap();
                    let cache = DiscoveryCache::new();
                    (cache, items.clone(), dir)
                },
                |(cache, items, _dir)| {
                    for (i, chunk) in items.chunks(100).enumerate() {
                        let path = write_test_file(&_dir, i);
                        cache.update(path, chunk.to_vec()).unwrap();
                    }
                    cache.save(&_dir.path().join(".fastest_cache")).unwrap();
                },
                criterion::BatchSize::SmallInput,
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
                    let cache = DiscoveryCache::new();
                    let items = create_test_items(size);

                    let mut files = Vec::new();

                    // Save items to cache
                    for (i, chunk) in items.chunks(100).enumerate() {
                        let path = write_test_file(&dir, i);
                        cache.update(path.clone(), chunk.to_vec()).unwrap();
                        files.push(path);
                    }
                    let cache_path = dir.path().join(".fastest_cache");
                    cache.save(&cache_path).unwrap();

                    (cache_path, files, dir)
                },
                |(cache_path, files, _dir)| {
                    // Benchmark: load all items from cache
                    let cache = DiscoveryCache::load(&cache_path).unwrap();
                    for path in &files {
                        let _ = black_box(cache.get(path));
                    }
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn benchmark_cache_has_valid_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_validation");

    group.bench_function("check_1000_files", |b| {
        let dir = TempDir::new().unwrap();
        let cache = DiscoveryCache::new();
        let items = create_test_items(100);
        let mut files = Vec::new();

        // Pre-populate cache
        for i in 0..1000 {
            let path = write_test_file(&dir, i);
            cache.update(path.clone(), items.clone()).unwrap();
            files.push(path);
        }
        cache.save(&dir.path().join(".fastest_cache")).unwrap();

        b.iter(|| {
            for path in &files {
                let _ = black_box(cache.get(path));
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
                let cache = DiscoveryCache::new();
                (cache, items.clone(), dir)
            },
            |(cache, items, _dir)| {
                // Save with compression
                let path = write_test_file(&_dir, 0);
                cache.update(path, items).unwrap();
                cache.save(&_dir.path().join(".fastest_cache")).unwrap();
            },
            criterion::BatchSize::SmallInput,
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
