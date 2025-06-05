//! 🚀 REVOLUTIONARY UTILITY MODULES
//! 
//! High-performance utility modules for optimizing execution engine performance.

// Re-export simd_json utilities from fastest_core
pub use fastest_core::utils::simd_json::{
    init_simd_json, is_simd_json_available, to_string, from_str, from_slice, 
    to_string_pretty, benchmark_json_performance, SimdJsonConfig, SimdJsonStats,
    init_simd_json_with_config, SIMDResult
};