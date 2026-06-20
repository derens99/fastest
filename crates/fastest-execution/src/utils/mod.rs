//! 🚀 REVOLUTIONARY UTILITY MODULES
//!
//! High-performance utility modules for optimizing execution engine performance.

// Re-export simd_json utilities from fastest_core
pub use fastest_core::utils::simd_json::{
    from_reader, from_slice, from_str, get_stats, initialize_simd, is_simd_available, to_string,
    to_string_pretty, to_vec, to_writer, JsonResult,
};
