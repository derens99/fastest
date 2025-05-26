#!/usr/bin/env python3
"""
Run scalability benchmark with progress monitoring
"""

import subprocess
import sys
import time
from pathlib import Path

def main():
    """Run the benchmark with progress updates"""
    print("🚀 Starting Scalability Benchmark")
    print("=" * 50)
    
    # Ensure we have the right environment
    venv_path = "/Users/derensnonwork/Desktop/test-fastest-project/venv/bin"
    fastest_path = f"{venv_path}/fastest"
    
    # Check if fastest is available
    try:
        result = subprocess.run([fastest_path, "version"], capture_output=True, text=True)
        if result.returncode == 0:
            print(f"✓ Using Fastest from: {fastest_path}")
            print(f"  Version: {result.stdout.strip()}")
        else:
            print(f"✗ Fastest not found at {fastest_path}")
            return
    except Exception as e:
        print(f"Error checking fastest: {e}")
        return
    
    # Set up environment
    import os
    env = os.environ.copy()
    env["PATH"] = f"{venv_path}:{env['PATH']}"
    
    print("\nRunning benchmark...")
    print("This will test scales: 10, 50, 100, 250, 500, 1000, 2500, 5000, 10000 tests")
    print("-" * 50)
    
    # Run the benchmark
    benchmark_script = Path(__file__).parent / "benchmark_scalability.py"
    
    try:
        # Run with line buffering for real-time output
        process = subprocess.Popen(
            [sys.executable, str(benchmark_script)],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            bufsize=1,
            env=env
        )
        
        # Stream output
        for line in process.stdout:
            print(line, end='')
        
        # Wait for completion
        process.wait()
        
        if process.returncode == 0:
            print("\n✅ Benchmark completed successfully!")
            
            # Check for output files
            output_dir = Path(__file__).parent
            png_file = output_dir / "scalability_benchmark.png"
            json_file = output_dir / "scalability_results.json"
            
            if png_file.exists():
                print(f"\n📊 Results saved:")
                print(f"  - Chart: {png_file}")
            if json_file.exists():
                print(f"  - Data: {json_file}")
        else:
            print(f"\n❌ Benchmark failed with exit code {process.returncode}")
            
    except KeyboardInterrupt:
        print("\n\n⚠️  Benchmark interrupted by user")
        process.terminate()
    except Exception as e:
        print(f"\n❌ Error running benchmark: {e}")

if __name__ == "__main__":
    main() 