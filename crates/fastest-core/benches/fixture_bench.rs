//! Fixture Management Performance Benchmarks
//!
//! Benchmarks for fixture system performance

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use fastest_core::test::fixtures::advanced::{
    AdvancedFixtureManager, FixtureDefinition, FixtureRequest, FixtureScope
};
use std::path::PathBuf;
use std::sync::Arc;

/// Create fixture definitions for benchmarking
fn create_fixtures(count: usize, dependencies_per_fixture: usize) -> Vec<FixtureDefinition> {
    (0..count)
        .map(|i| {
            let deps = if i > dependencies_per_fixture {
                (0..dependencies_per_fixture)
                    .map(|j| Arc::from(format!("fixture_{}", i - j - 1)))
                    .collect()
            } else {
                smallvec::smallvec![]
            };
            
            FixtureDefinition {
                name: Arc::from(format!("fixture_{}", i)),
                scope: match i % 4 {
                    0 => FixtureScope::Function,
                    1 => FixtureScope::Class,
                    2 => FixtureScope::Module,
                    _ => FixtureScope::Session,
                },
                flags: if i % 10 == 0 { 0x01 } else { 0 }, // Some autouse
                params: smallvec::smallvec![],
                ids: smallvec::smallvec![],
                dependencies: deps,
                module_path: Arc::new(PathBuf::from("test_module.py")),
                line_number: (i + 1) as u32,
            }
        })
        .collect()
}

/// Create a fixture request
fn create_fixture_request(requested_fixtures: Vec<String>) -> FixtureRequest {
    FixtureRequest {
        node_id: Arc::from("test_module.py::test_function"),
        test_name: Arc::from("test_function"),
        module_path: Arc::new(PathBuf::from("test_module.py")),
        class_name: None,
        param_index: 0,
        requested_fixtures: requested_fixtures.into_iter()
            .map(|s| Arc::from(s.as_str()))
            .collect(),
        indirect_params: rustc_hash::FxHashMap::default(),
    }
}

fn benchmark_fixture_registration(c: &mut Criterion) {
    let mut group = c.benchmark_group("fixture_registration");
    
    for size in [100, 500, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let fixtures = create_fixtures(size, 3);
            
            b.iter(|| {
                let manager = AdvancedFixtureManager::new();
                for fixture in &fixtures {
                    let _ = manager.register_fixture(fixture.clone());
                }
            });
        });
    }
    
    group.finish();
}

fn benchmark_fixture_dependency_resolution(c: &mut Criterion) {
    let mut group = c.benchmark_group("fixture_dependencies");
    
    group.bench_function("100_fixtures_deep_deps", |b| {
        let manager = AdvancedFixtureManager::new();
        let fixtures = create_fixtures(100, 5);
        
        // Register all fixtures
        for fixture in &fixtures {
            manager.register_fixture(fixture.clone()).unwrap();
        }
        
        // Request that uses many fixtures
        let request = create_fixture_request(
            (90..100).map(|i| format!("fixture_{}", i)).collect()
        );
        
        b.iter(|| {
            let _ = black_box(manager.get_required_fixtures(&request));
        });
    });
    
    group.finish();
}

fn benchmark_fixture_setup(c: &mut Criterion) {
    let mut group = c.benchmark_group("fixture_setup");
    
    for num_fixtures in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_fixtures), 
            num_fixtures, 
            |b, &num_fixtures| {
                let manager = AdvancedFixtureManager::new();
                let fixtures = create_fixtures(num_fixtures * 2, 2);
                
                // Register all fixtures
                for fixture in &fixtures {
                    manager.register_fixture(fixture.clone()).unwrap();
                }
                
                // Request subset of fixtures
                let request = create_fixture_request(
                    (0..num_fixtures).map(|i| format!("fixture_{}", i * 2)).collect()
                );
                
                b.iter(|| {
                    let _ = black_box(manager.setup_fixtures(&request));
                });
            }
        );
    }
    
    group.finish();
}

fn benchmark_fixture_teardown(c: &mut Criterion) {
    let mut group = c.benchmark_group("fixture_teardown");
    
    group.bench_function("teardown_50_fixtures", |b| {
        let manager = AdvancedFixtureManager::new();
        let fixtures = create_fixtures(50, 2);
        
        // Register all fixtures
        for fixture in &fixtures {
            manager.register_fixture(fixture.clone()).unwrap();
        }
        
        b.iter_batched(
            || {
                // Setup: create fixtures
                let request = create_fixture_request(
                    (0..50).map(|i| format!("fixture_{}", i)).collect()
                );
                manager.setup_fixtures(&request).unwrap();
                request
            },
            |request| {
                // Benchmark: teardown fixtures
                manager.teardown_fixtures(&request, FixtureScope::Function).unwrap();
            },
            criterion::BatchSize::SmallInput
        );
    });
    
    group.finish();
}

fn benchmark_autouse_fixture_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("fixture_autouse");
    
    group.bench_function("detect_autouse_from_1000", |b| {
        let manager = AdvancedFixtureManager::new();
        
        // Create fixtures with 10% autouse
        let fixtures: Vec<_> = (0..1000)
            .map(|i| FixtureDefinition {
                name: Arc::from(format!("fixture_{}", i)),
                scope: FixtureScope::Function,
                flags: if i % 10 == 0 { 0x01 } else { 0 },
                params: smallvec::smallvec![],
                ids: smallvec::smallvec![],
                dependencies: smallvec::smallvec![],
                module_path: Arc::new(PathBuf::from("test_module.py")),
                line_number: (i + 1) as u32,
            })
            .collect();
        
        // Register all fixtures
        for fixture in &fixtures {
            manager.register_fixture(fixture.clone()).unwrap();
        }
        
        let request = create_fixture_request(vec!["explicit_fixture".to_string()]);
        
        b.iter(|| {
            let _ = black_box(manager.get_required_fixtures(&request));
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_fixture_registration,
    benchmark_fixture_dependency_resolution,
    benchmark_fixture_setup,
    benchmark_fixture_teardown,
    benchmark_autouse_fixture_detection
);
criterion_main!(benches);