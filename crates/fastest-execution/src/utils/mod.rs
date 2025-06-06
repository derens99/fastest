//! 🚀 REVOLUTIONARY UTILITY MODULES
//!
//! High-performance utility modules for optimizing execution engine performance.

// Re-export simd_json utilities from fastest_core
pub use fastest_core::utils::simd_json::{
    benchmark_json_performance, from_slice, from_str, init_simd_json, init_simd_json_with_config,
    is_simd_json_available, to_string, to_string_pretty, SIMDResult, SimdJsonConfig, SimdJsonStats,
};
