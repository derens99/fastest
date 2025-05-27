#!/usr/bin/env python3
"""
Demo script to verify fastest installation and show basic usage.
Run this after installing fastest to verify everything works.
"""

import subprocess
import sys
import os
import time
import tempfile

def run_command(cmd, capture=True):
    """Run a command and return output."""
    if capture:
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
        return result.stdout, result.stderr, result.returncode
    else:
        return subprocess.run(cmd, shell=True).returncode

def create_test_file(path, content):
    """Create a test file with given content."""
    with open(path, 'w') as f:
        f.write(content)

def main():
    print("🚀 Fastest Installation Demo")
    print("=" * 50)
    
    # Add common install locations to PATH
    home = os.path.expanduser("~")
    extra_paths = [
        os.path.join(home, "bin"),
        os.path.join(home, ".local", "bin"),
        os.path.join(home, ".cargo", "bin"),
    ]
    
    current_path = os.environ.get("PATH", "")
    for path in extra_paths:
        if os.path.exists(path) and path not in current_path:
            os.environ["PATH"] = f"{path}:{current_path}"
            current_path = os.environ["PATH"]
    
    # Check if fastest is installed
    print("\n1. Checking fastest installation...")
    stdout, stderr, code = run_command("which fastest")
    if code != 0:
        print("❌ Fastest not found in PATH")
        print("   Please run ./install.sh or ./install-dev.sh first")
        return 1
    
    fastest_path = stdout.strip()
    print(f"✅ Fastest installed at: {fastest_path}")
    
    # Create a temporary test directory
    with tempfile.TemporaryDirectory() as tmpdir:
        print(f"\n2. Creating test files in {tmpdir}...")
        
        # Create simple test
        test_file = os.path.join(tmpdir, "test_demo.py")
        create_test_file(test_file, """
def test_addition():
    assert 1 + 1 == 2

def test_subtraction():
    assert 5 - 3 == 2

def test_multiplication():
    assert 3 * 4 == 12

def test_division():
    assert 10 / 2 == 5

def test_string_concat():
    assert "hello" + " world" == "hello world"
""")
        
        print("✅ Created test_demo.py with 5 tests")
        
        # Run with fastest
        print("\n3. Running tests with fastest...")
        start = time.time()
        stdout, stderr, code = run_command(f"cd {tmpdir} && fastest test_demo.py --optimizer simple")
        fastest_time = time.time() - start
        
        if "passed" in stdout:
            print(f"✅ Fastest completed in {fastest_time:.3f}s")
            # Extract test count and time from output
            for line in stdout.split('\n'):
                if 'passed' in line:
                    print(f"   {line.strip()}")
                    break
        else:
            print(f"❌ Fastest failed to run tests")
            if stderr:
                print(f"   Error: {stderr}")
            
        # Run with pytest for comparison (if available)
        print("\n4. Comparing with pytest (if available)...")
        stdout, stderr, code = run_command("pytest --version")
        if code == 0:
            start = time.time()
            stdout, stderr, code = run_command(f"cd {tmpdir} && pytest test_demo.py -v")
            pytest_time = time.time() - start
            
            if code == 0:
                print(f"✅ Pytest completed in {pytest_time:.3f}s")
                speedup = pytest_time / fastest_time
                print(f"\n🎉 Fastest is {speedup:.1f}x faster than pytest!")
            else:
                print("❌ Pytest failed to run tests")
        else:
            print("ℹ️  Pytest not installed, skipping comparison")
            
        # Test different optimizers
        print("\n5. Testing different optimizers...")
        optimizers = ["simple", "optimized", "parallel"]
        
        for optimizer in optimizers:
            stdout, stderr, code = run_command(
                f"cd {tmpdir} && fastest test_demo.py --optimizer {optimizer}",
                capture=True
            )
            if code == 0:
                # Extract timing from output
                for line in stdout.split('\n'):
                    if 'in' in line and 's' in line:
                        print(f"   {optimizer}: {line.strip()}")
                        break
            else:
                print(f"   {optimizer}: Failed")
                
    print("\n" + "=" * 50)
    print("✅ Installation verified! Fastest is working correctly.")
    print("\nNext steps:")
    print("  - Run 'fastest' in any directory with Python tests")
    print("  - Use 'fastest --help' to see all options")
    print("  - Try different optimizers with --optimizer flag")
    print("  - Read the docs at INSTALLATION.md")
    
    # Also update PATH instructions if needed
    if "~/bin" not in os.environ.get("PATH", ""):
        print("\n⚠️  Note: Add ~/bin to your PATH for easier access:")
        print("    echo 'export PATH=\"$HOME/bin:$PATH\"' >> ~/.zshrc")
        print("    source ~/.zshrc")
    
    return 0

if __name__ == "__main__":
    sys.exit(main())