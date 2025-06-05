pub mod python;

/// 🚀 REVOLUTIONARY SIMD-accelerated JSON processing
pub mod simd_json;

pub use python::{detect_python_command, get_python_version, PYTHON_CMD};

// Re-export SIMD JSON for convenience
pub use simd_json::*;
