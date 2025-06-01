#!/bin/bash
# Comprehensive benchmark and chart generation script
# Runs official benchmarks and generates performance visualization charts

set -e  # Exit on any error

echo "🚀 Fastest - Complete Performance Analysis"
echo "=========================================="

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Please run this script from the project root directory"
    exit 1
fi

# Build release version
echo "🔨 Building release version..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "❌ Build failed!"
    exit 1
fi

echo "✅ Build successful"

# Check for required dependencies
echo "🔍 Checking dependencies..."

if ! python -c "import matplotlib" 2>/dev/null; then
    echo "❌ matplotlib not found. Installing..."
    pip install matplotlib seaborn
fi

echo "✅ Dependencies verified"

# Run official benchmark
echo "📊 Running official performance benchmark..."
if [ "$1" = "--quick" ]; then
    python scripts/official_benchmark.py --quick
else
    python scripts/official_benchmark.py
fi

if [ $? -ne 0 ]; then
    echo "❌ Benchmark failed!"
    exit 1
fi

echo "✅ Benchmark completed"

# Generate performance charts
echo "🎨 Generating performance charts..."
python scripts/generate_charts.py

if [ $? -ne 0 ]; then
    echo "❌ Chart generation failed!"
    exit 1
fi

echo "✅ Charts generated"

# Show results
echo ""
echo "=========================================="
echo "🎉 ANALYSIS COMPLETE!"
echo "=========================================="
echo ""
echo "📊 Benchmark Results:"
echo "  - benchmarks/official_results.json"
echo "  - docs/OFFICIAL_BENCHMARK_RESULTS.md"
echo ""
echo "🎨 Performance Charts:"
echo "  - docs/images/performance_comparison.png"
echo "  - docs/images/scaling_analysis.png"
echo "  - docs/images/performance_summary.png"
echo ""
echo "🔍 To view results:"
echo "  cat docs/OFFICIAL_BENCHMARK_RESULTS.md"
echo "  open docs/images/performance_comparison.png"
echo ""

# Extract and display key metrics
if [ -f "benchmarks/official_results.json" ]; then
    if command -v jq &> /dev/null; then
        echo "📈 Key Performance Metrics:"
        echo "  Average Speedup: $(jq -r '.summary.avg_total_speedup // "N/A"' benchmarks/official_results.json | xargs printf "%.1f")x"
        echo "  Maximum Speedup: $(jq -r '.summary.max_total_speedup // "N/A"' benchmarks/official_results.json | xargs printf "%.1f")x"
        echo "  Test Sizes: $(jq -r '.summary.test_suite_sizes_tested // "N/A"' benchmarks/official_results.json) different sizes"
    else
        echo "💡 Install 'jq' to see performance metrics summary"
    fi
fi

echo ""
echo "🚀 Performance analysis complete! Check the charts in docs/images/"