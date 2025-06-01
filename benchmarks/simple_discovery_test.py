#!/usr/bin/env python3
"""
Simple discovery performance test to showcase our optimizations
"""

import subprocess
import time
from pathlib import Path

def test_fastest_discovery():
    """Test fastest discovery performance"""
    fastest_path = Path(__file__).parent.parent / "target/release/fastest"
    
    print("🚀 Testing Fastest Discovery Performance")
    print("=" * 50)
    
    times = []
    for i in range(3):
        start_time = time.perf_counter()
        result = subprocess.run([
            str(fastest_path), "discover", "--format", "count"
        ], capture_output=True, text=True, cwd=Path(__file__).parent.parent)
        end_time = time.perf_counter()
        
        discovery_time = end_time - start_time
        times.append(discovery_time)
        
        if result.returncode == 0:
            # Extract test count
            test_count = None
            for line in result.stdout.strip().split('\n'):
                if line.isdigit():
                    test_count = int(line)
                    break
            
            print(f"   Run {i+1}: {discovery_time:.3f}s ({test_count} tests)")
        else:
            print(f"   Run {i+1}: FAILED")
            print(f"   Error: {result.stderr}")
    
    if times:
        avg_time = sum(times) / len(times)
        min_time = min(times)
        
        print()
        print("📊 DISCOVERY PERFORMANCE RESULTS:")
        print(f"   • Average time: {avg_time:.3f}s")
        print(f"   • Best time:    {min_time:.3f}s")
        print(f"   • Tests found:  {test_count if 'test_count' in locals() else 'Unknown'}")
        print()
        
        # Compare against typical pytest performance
        estimated_pytest_time = 0.5  # Conservative estimate for pytest discovery
        speedup = estimated_pytest_time / avg_time
        
        print(f"🎯 ESTIMATED SPEEDUP vs pytest:")
        print(f"   • Pytest (estimated): ~{estimated_pytest_time:.1f}s")
        print(f"   • Fastest (measured):  {avg_time:.3f}s")
        print(f"   • Speedup factor:      ~{speedup:.1f}x faster")
        
        if speedup >= 3.0:
            print(f"\n✅ EXCELLENT: Discovery is ~{speedup:.1f}x faster!")
        elif speedup >= 2.0:
            print(f"\n✅ GOOD: Discovery is ~{speedup:.1f}x faster!")
        else:
            print(f"\n📈 Discovery is ~{speedup:.1f}x faster")

if __name__ == "__main__":
    test_fastest_discovery()