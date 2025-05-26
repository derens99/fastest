#!/usr/bin/env python3
"""Format benchmark results for GitHub benchmark action.

The github-action-benchmark expects JSON in this format:
[
  {
    "name": "Benchmark Name",
    "unit": "ms",
    "value": 123.45
  },
  ...
]
"""

import json
import subprocess
import sys
import time

def run_benchmark_and_extract_metrics():
    """Run benchmarks and extract key metrics for GitHub action."""
    results = []
    
    # Run the main benchmark script that generates our custom format
    result = subprocess.run([sys.executable, "benchmarks/format_benchmark_json.py"], 
                          capture_output=True, text=True)
    
    if result.returncode == 0:
        try:
            data = json.loads(result.stdout)
            
            # Extract discovery speedup
            discovery = data.get('benchmarks', {}).get('discovery', {})
            if 'discovery_speedup' in discovery:
                results.append({
                    "name": "Discovery Speedup vs pytest",
                    "unit": "x faster",
                    "value": discovery['discovery_speedup']
                })
            
            # Extract execution speedup
            if 'execution_speedup' in discovery:
                results.append({
                    "name": "Execution Speedup vs pytest",
                    "unit": "x faster", 
                    "value": discovery['execution_speedup']
                })
            
            # Extract parser benchmarks
            parser = data.get('benchmarks', {}).get('parser', {})
            
            # Find AST speedups for different test counts
            for key, value in parser.items():
                if key.startswith('ast_speedup_') and key.endswith('_tests'):
                    test_count = key.split('_')[2]
                    results.append({
                        "name": f"AST Parser Speedup ({test_count} tests)",
                        "unit": "x faster",
                        "value": value
                    })
            
            # Extract parallel execution speedup
            parallel = data.get('benchmarks', {}).get('parallel', {})
            if 'parallel_speedup' in parallel:
                results.append({
                    "name": "Parallel Execution Speedup",
                    "unit": "x faster",
                    "value": parallel['parallel_speedup']
                })
            
            # Add test discovery counts if available
            if 'fastest_test_count' in discovery:
                results.append({
                    "name": "Tests Discovered",
                    "unit": "tests",
                    "value": discovery['fastest_test_count']
                })
                
        except json.JSONDecodeError:
            print(f"Error: Could not parse benchmark JSON output", file=sys.stderr)
    else:
        print(f"Error: Benchmark script failed with code {result.returncode}", file=sys.stderr)
        print(f"Stderr: {result.stderr}", file=sys.stderr)
    
    # If no results were extracted, add a dummy result to avoid empty array
    if not results:
        results.append({
            "name": "Benchmark Status",
            "unit": "success",
            "value": 0  # 0 indicates failure
        })
    
    return results

def main():
    """Main function to output benchmark results for GitHub action."""
    results = run_benchmark_and_extract_metrics()
    
    # Output JSON array as expected by github-action-benchmark
    print(json.dumps(results, indent=2))

if __name__ == "__main__":
    main() 