#!/usr/bin/env python3
"""Format benchmark results as JSON for GitHub Actions."""

import json
import subprocess
import sys
import time

def run_benchmark(script_name):
    """Run a benchmark script and capture its output."""
    start = time.time()
    result = subprocess.run([sys.executable, f"benchmarks/{script_name}"], 
                          capture_output=True, text=True)
    duration = time.time() - start
    return result.stdout, result.stderr, duration

def parse_discovery_benchmark(output):
    """Parse discovery benchmark output."""
    results = {}
    for line in output.split('\n'):
        if 'discovered' in line and 'tests in' in line:
            # Extract number of tests and time
            parts = line.split()
            if len(parts) >= 5:
                test_count = int(parts[1])
                time_ms = float(parts[4].rstrip('ms'))
                tool = 'fastest' if 'Fastest' in line else 'pytest'
                results[f'{tool}_discovery_time_ms'] = time_ms
                results[f'{tool}_discovery_count'] = test_count
        elif 'x faster' in line:
            parts = line.split()
            speedup = float(parts[0].rstrip('x'))
            results['discovery_speedup'] = speedup
    return results

def parse_parser_benchmark(output):
    """Parse parser benchmark output."""
    results = {}
    current_size = None
    
    for line in output.split('\n'):
        if 'Test suite size:' in line:
            current_size = int(line.split()[-1])
        elif 'Regex parser:' in line and 'ms' in line:
            time_ms = float(line.split()[-1].rstrip('ms'))
            results[f'regex_parser_{current_size}_tests_ms'] = time_ms
        elif 'AST parser:' in line and 'ms' in line:
            time_ms = float(line.split()[-1].rstrip('ms'))
            results[f'ast_parser_{current_size}_tests_ms'] = time_ms
        elif 'AST parser is' in line and 'faster' in line:
            speedup = float(line.split()[3].rstrip('x'))
            results[f'ast_speedup_{current_size}_tests'] = speedup
    
    return results

def parse_parallel_benchmark(output):
    """Parse parallel execution benchmark output."""
    results = {}
    
    for line in output.split('\n'):
        if 'Sequential:' in line:
            time_s = float(line.split()[-1].rstrip('s'))
            results['sequential_execution_s'] = time_s
        elif 'Parallel' in line and 'workers' in line and ':' in line:
            parts = line.split()
            workers = int(parts[1].strip('('))
            time_s = float(parts[-1].rstrip('s'))
            results[f'parallel_{workers}_workers_s'] = time_s
        elif 'Speedup with' in line:
            parts = line.split()
            workers = int(parts[2])
            speedup = float(parts[-1].rstrip('x'))
            results[f'speedup_{workers}_workers'] = speedup
    
    return results

def main():
    """Run all benchmarks and output JSON results."""
    all_results = {
        'timestamp': int(time.time()),
        'benchmarks': {}
    }
    
    # Run discovery benchmark
    print("Running discovery benchmark...", file=sys.stderr)
    output, _, duration = run_benchmark("benchmark.py")
    discovery_results = parse_discovery_benchmark(output)
    discovery_results['benchmark_duration_s'] = duration
    all_results['benchmarks']['discovery'] = discovery_results
    
    # Run parser benchmark
    print("Running parser benchmark...", file=sys.stderr)
    output, _, duration = run_benchmark("benchmark_parsers.py")
    parser_results = parse_parser_benchmark(output)
    parser_results['benchmark_duration_s'] = duration
    all_results['benchmarks']['parser'] = parser_results
    
    # Run parallel benchmark
    print("Running parallel execution benchmark...", file=sys.stderr)
    output, _, duration = run_benchmark("benchmark_parallel.py")
    parallel_results = parse_parallel_benchmark(output)
    parallel_results['benchmark_duration_s'] = duration
    all_results['benchmarks']['parallel'] = parallel_results
    
    # Calculate overall metrics
    if 'discovery_speedup' in discovery_results:
        all_results['summary'] = {
            'discovery_speedup': discovery_results.get('discovery_speedup', 0),
            'best_parser_speedup': max(
                v for k, v in parser_results.items() 
                if k.startswith('ast_speedup')
            ) if any(k.startswith('ast_speedup') for k in parser_results) else 0,
            'best_parallel_speedup': max(
                v for k, v in parallel_results.items() 
                if k.startswith('speedup_')
            ) if any(k.startswith('speedup_') for k in parallel_results) else 0
        }
    
    # Output JSON
    print(json.dumps(all_results, indent=2))

if __name__ == "__main__":
    main() 