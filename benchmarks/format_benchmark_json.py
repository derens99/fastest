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
        line = line.strip()
        
        # Parse "Discovery: fastest is X.XXx faster than pytest"
        if 'Discovery:' in line and 'faster than pytest' in line:
            try:
                # Extract the number before 'x faster'
                import re
                match = re.search(r'(\d+\.?\d*)x faster', line)
                if match:
                    speedup = float(match.group(1))
                    results['discovery_speedup'] = speedup
            except (ValueError, AttributeError):
                pass
        
        # Parse "Execution: fastest is X.XXx faster than pytest"
        elif 'Execution:' in line and 'faster than pytest' in line:
            try:
                import re
                match = re.search(r'(\d+\.?\d*)x faster', line)
                if match:
                    speedup = float(match.group(1))
                    results['execution_speedup'] = speedup
            except (ValueError, AttributeError):
                pass
        
        # Parse test counts and times from table output
        elif 'avg_time' in line or 'min_time' in line or 'max_time' in line:
            # Skip table headers and parse data rows
            continue
        elif 'Tests found' in line:
            # Parse "Tests found    100    100"
            parts = line.split()
            if len(parts) >= 3:
                try:
                    pytest_count = int(parts[-2])
                    fastest_count = int(parts[-1])
                    results['pytest_test_count'] = pytest_count
                    results['fastest_test_count'] = fastest_count
                except ValueError:
                    pass
    
    return results

def parse_parser_benchmark(output):
    """Parse parser benchmark output."""
    results = {}
    current_tests = None
    
    for line in output.split('\n'):
        line = line.strip()
        
        # Parse test count from benchmark headers
        if 'Benchmarking with' in line and 'tests' in line:
            import re
            match = re.search(r'~(\d+) tests', line)
            if match:
                current_tests = int(match.group(1))
        
        # Parse parser results
        elif 'Found' in line and 'tests in' in line and 's' in line:
            # Parse "✓ Found 1000 tests in 0.123s"
            parts = line.split()
            try:
                test_count = int(parts[2])
                time_str = parts[-1].rstrip('s')
                time_s = float(time_str)
                
                if 'regex' in line.lower():
                    results[f'regex_parser_{current_tests}_tests_s'] = time_s
                    results[f'regex_parser_{current_tests}_count'] = test_count
                elif 'ast' in line.lower():
                    results[f'ast_parser_{current_tests}_tests_s'] = time_s
                    results[f'ast_parser_{current_tests}_count'] = test_count
            except (ValueError, IndexError):
                pass
        
        # Parse speedup comparisons
        elif 'parser is' in line and 'faster' in line:
            try:
                import re
                match = re.search(r'(\d+\.?\d*)x faster', line)
                if match:
                    speedup = float(match.group(1))
                    if 'AST' in line:
                        results[f'ast_speedup_{current_tests}_tests'] = speedup
                    elif 'Regex' in line:
                        results[f'regex_speedup_{current_tests}_tests'] = speedup
            except (ValueError, AttributeError):
                pass
    
    return results

def parse_parallel_benchmark(output):
    """Parse parallel execution benchmark output."""
    results = {}
    
    for line in output.split('\n'):
        line = line.strip()
        
        # Parse sequential execution time
        if 'Sequential:' in line and 's' in line:
            try:
                import re
                match = re.search(r'(\d+\.?\d*)s', line)
                if match:
                    time_s = float(match.group(1))
                    results['sequential_execution_s'] = time_s
            except (ValueError, AttributeError):
                pass
        
        # Parse parallel execution times
        elif 'Parallel:' in line and 's' in line:
            try:
                import re
                match = re.search(r'(\d+\.?\d*)s', line)
                if match:
                    time_s = float(match.group(1))
                    results['parallel_execution_s'] = time_s
            except (ValueError, AttributeError):
                pass
        
        # Parse speedup information
        elif 'Speedup:' in line and 'x' in line:
            try:
                import re
                match = re.search(r'(\d+\.?\d*)x', line)
                if match:
                    speedup = float(match.group(1))
                    results['parallel_speedup'] = speedup
            except (ValueError, AttributeError):
                pass
        
        # Parse worker-specific results
        elif 'workers):' in line and 'Time:' in line:
            try:
                # Parse worker count and time from lines like "2. Parallel execution (4 workers):"
                import re
                worker_match = re.search(r'\((\d+) workers\)', line)
                time_match = re.search(r'(\d+\.?\d*)s', line)
                if worker_match and time_match:
                    workers = int(worker_match.group(1))
                    time_s = float(time_match.group(1))
                    results[f'parallel_{workers}_workers_s'] = time_s
            except (ValueError, AttributeError):
                pass
    
    return results

def main():
    """Run all benchmarks and output JSON results."""
    all_results = {
        'timestamp': int(time.time()),
        'benchmarks': {}
    }
    
    # Run discovery benchmark
    print("Running discovery benchmark...", file=sys.stderr)
    try:
        output, stderr, duration = run_benchmark("benchmark.py")
        if stderr:
            print(f"Discovery benchmark stderr: {stderr}", file=sys.stderr)
        discovery_results = parse_discovery_benchmark(output)
        discovery_results['benchmark_duration_s'] = duration
        all_results['benchmarks']['discovery'] = discovery_results
    except Exception as e:
        print(f"Discovery benchmark failed: {e}", file=sys.stderr)
        all_results['benchmarks']['discovery'] = {'error': str(e)}
    
    # Run parser benchmark
    print("Running parser benchmark...", file=sys.stderr)
    try:
        output, stderr, duration = run_benchmark("benchmark_parsers.py")
        if stderr:
            print(f"Parser benchmark stderr: {stderr}", file=sys.stderr)
        parser_results = parse_parser_benchmark(output)
        parser_results['benchmark_duration_s'] = duration
        all_results['benchmarks']['parser'] = parser_results
    except Exception as e:
        print(f"Parser benchmark failed: {e}", file=sys.stderr)
        all_results['benchmarks']['parser'] = {'error': str(e)}
    
    # Run parallel benchmark
    print("Running parallel execution benchmark...", file=sys.stderr)
    try:
        output, stderr, duration = run_benchmark("benchmark_parallel.py")
        if stderr:
            print(f"Parallel benchmark stderr: {stderr}", file=sys.stderr)
        parallel_results = parse_parallel_benchmark(output)
        parallel_results['benchmark_duration_s'] = duration
        all_results['benchmarks']['parallel'] = parallel_results
    except Exception as e:
        print(f"Parallel benchmark failed: {e}", file=sys.stderr)
        all_results['benchmarks']['parallel'] = {'error': str(e)}
    
    # Calculate overall metrics
    discovery_results = all_results['benchmarks'].get('discovery', {})
    parser_results = all_results['benchmarks'].get('parser', {})
    parallel_results = all_results['benchmarks'].get('parallel', {})
    
    all_results['summary'] = {
        'discovery_speedup': discovery_results.get('discovery_speedup', 0),
        'best_parser_speedup': max(
            v for k, v in parser_results.items() 
            if k.startswith('ast_speedup') and isinstance(v, (int, float))
        ) if any(k.startswith('ast_speedup') for k in parser_results) else 0,
        'best_parallel_speedup': max([
            parallel_results.get('parallel_speedup', 0),
            *[v for k, v in parallel_results.items() 
              if k.startswith('speedup_') and isinstance(v, (int, float))]
        ]) if parallel_results else 0
    }
    
    # Output JSON
    print(json.dumps(all_results, indent=2))

if __name__ == "__main__":
    main() 