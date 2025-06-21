#!/bin/bash
# Run performance benchmarks for fastest-core

set -e

echo "🚀 Running fastest-core performance benchmarks..."
echo "================================================"

# Ensure we're in the project root
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"
cd "$PROJECT_ROOT"

# Build in release mode first
echo "📦 Building fastest-core in release mode..."
cargo build --release -p fastest-core

# Run benchmarks
echo ""
echo "🏃 Running discovery benchmarks..."
cargo bench -p fastest-core --bench discovery_bench

echo ""
echo "💾 Running cache benchmarks..."
cargo bench -p fastest-core --bench cache_bench

echo ""
echo "🔍 Running parser benchmarks..."
cargo bench -p fastest-core --bench parser_bench

echo ""
echo "🧩 Running fixture benchmarks..."
cargo bench -p fastest-core --bench fixture_bench

echo ""
echo "⚡ Running SIMD JSON benchmarks..."
cargo bench -p fastest-core --bench simd_json_bench

echo ""
echo "✅ All benchmarks complete!"
echo ""
echo "📊 Benchmark reports available in:"
echo "   target/criterion/"
echo ""
echo "To view HTML reports, open:"
echo "   target/criterion/report/index.html"