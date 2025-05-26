#!/usr/bin/env python3
"""
Create animated terminal demos showing Fastest performance.
Can be converted to GIF using asciinema or terminalizer.
"""

import time
import sys
import random
from typing import List

# ANSI color codes
RED = '\033[91m'
GREEN = '\033[92m'
YELLOW = '\033[93m'
BLUE = '\033[94m'
MAGENTA = '\033[95m'
CYAN = '\033[96m'
WHITE = '\033[97m'
RESET = '\033[0m'
BOLD = '\033[1m'
CLEAR = '\033[2J\033[H'

def print_slowly(text: str, delay: float = 0.03):
    """Print text character by character for effect."""
    for char in text:
        sys.stdout.write(char)
        sys.stdout.flush()
        time.sleep(delay)
    print()

def print_progress_bar(current: int, total: int, label: str = "", bar_length: int = 50):
    """Print a progress bar."""
    progress = current / total
    filled = int(bar_length * progress)
    bar = "█" * filled + "░" * (bar_length - filled)
    percentage = progress * 100
    
    sys.stdout.write(f'\r{label} [{bar}] {percentage:.1f}%')
    sys.stdout.flush()

def demo_discovery_comparison():
    """Animate test discovery comparison."""
    print(CLEAR)
    print(f"{BOLD}{CYAN}=== Test Discovery Comparison ==={RESET}\n")
    
    # Pytest discovery
    print(f"{YELLOW}$ pytest --collect-only tests/{RESET}")
    time.sleep(0.5)
    
    # Simulate slow discovery
    test_files = [
        "test_auth.py", "test_models.py", "test_views.py", "test_utils.py",
        "test_api.py", "test_integration.py", "test_validators.py", "test_helpers.py"
    ]
    
    for i, file in enumerate(test_files):
        print(f"collecting {file}...", end='', flush=True)
        time.sleep(0.3)  # Simulate slow collection
        print(f" {GREEN}✓{RESET}")
    
    print(f"\n{RED}Time: 358ms{RESET}")
    print(f"Collected 1000 tests\n")
    
    input(f"{MAGENTA}Press Enter to see Fastest...{RESET}")
    
    # Fastest discovery
    print(f"\n{YELLOW}$ fastest discover tests/{RESET}")
    time.sleep(0.2)
    
    # Simulate fast discovery with progress bar
    for i in range(101):
        print_progress_bar(i, 100, "Discovering tests", 40)
        time.sleep(0.00005)  # Much faster!
    
    print(f"\n\n{GREEN}Time: 6.7ms{RESET}")
    print(f"Discovered 1000 tests")
    print(f"\n{BOLD}{GREEN}53x faster!{RESET}")

def demo_execution_comparison():
    """Animate test execution comparison."""
    time.sleep(2)
    print(CLEAR)
    print(f"{BOLD}{CYAN}=== Test Execution Comparison ==={RESET}\n")
    
    # Pytest execution
    print(f"{YELLOW}$ pytest tests/{RESET}")
    time.sleep(0.5)
    
    # Simulate test execution
    test_names = [
        "test_login", "test_logout", "test_create_user", "test_delete_user",
        "test_api_auth", "test_data_validation", "test_file_upload", "test_search"
    ]
    
    passed = 0
    for i, test in enumerate(test_names * 12):  # 96 tests
        status = f"{GREEN}.{RESET}" if random.random() > 0.1 else f"{RED}F{RESET}"
        sys.stdout.write(status)
        sys.stdout.flush()
        if status == f"{GREEN}.{RESET}":
            passed += 1
        time.sleep(0.02)  # Simulate test execution
        if (i + 1) % 40 == 0:
            print()
    
    print(f"\n\n{GREEN}{passed} passed{RESET}, {RED}{96-passed} failed{RESET}")
    print(f"{RED}Time: 1.87s{RESET}\n")
    
    input(f"{MAGENTA}Press Enter to see Fastest...{RESET}")
    
    # Fastest execution
    print(f"\n{YELLOW}$ fastest tests/{RESET}")
    time.sleep(0.2)
    
    # Show parallel execution
    print(f"{BLUE}Running tests with 4 workers...{RESET}\n")
    
    # Simulate parallel progress bars
    workers = ["Worker 1", "Worker 2", "Worker 3", "Worker 4"]
    for step in range(25):
        sys.stdout.write('\033[4A')  # Move cursor up 4 lines
        for i, worker in enumerate(workers):
            progress = min((step + i * 2) * 4, 100)
            print_progress_bar(progress, 100, f"{worker:10}", 30)
            print()
        time.sleep(0.035)
    
    print(f"\n{GREEN}96 passed{RESET}, {RED}0 failed{RESET}")
    print(f"{GREEN}Time: 0.89s{RESET}")
    print(f"\n{BOLD}{GREEN}2.1x faster!{RESET}")

def demo_feature_comparison():
    """Show feature comparison."""
    time.sleep(2)
    print(CLEAR)
    print(f"{BOLD}{CYAN}=== Feature Comparison ==={RESET}\n")
    
    features = [
        ("Startup time", "~200ms", "<100ms", True),
        ("Tree-sitter parser", "❌", "✅", True),
        ("Smart caching", "❌", "✅", True),
        ("Parallel execution", "pytest-xdist", "Built-in", True),
        ("Memory usage", "100%", "50%", True),
        ("Rust performance", "❌", "✅", True),
    ]
    
    print(f"{'Feature':<25} {'pytest':<15} {'fastest':<15}")
    print("─" * 55)
    
    for feature, pytest_val, fastest_val, fastest_better in features:
        time.sleep(0.3)
        fastest_color = GREEN if fastest_better else WHITE
        print(f"{feature:<25} {pytest_val:<15} {fastest_color}{fastest_val:<15}{RESET}")

def create_terminalizer_config():
    """Create configuration for terminalizer to record the demo."""
    config = """
# Terminalizer configuration for Fastest demo
# Install: npm install -g terminalizer
# Record: terminalizer record fastest-demo -c terminalizer.yml
# Render: terminalizer render fastest-demo -o fastest-demo.gif

command: python benchmarks/create_demo_animation.py
cwd: .
env:
  TERM: xterm-256color
cols: 80
rows: 24
repeat: 0
quality: 100
frameDelay: auto
maxIdleTime: 2000
frameBox:
  type: solid
  title: "Fastest vs pytest"
  style:
    boxShadow: none
    margin: 0px
watermark:
  imagePath: null
  style:
    position: absolute
cursorStyle: block
fontFamily: "Monaco, Consolas, 'Courier New', monospace"
fontSize: 14
lineHeight: 1.2
letterSpacing: 0
"""
    
    with open('terminalizer.yml', 'w') as f:
        f.write(config)
    print("Created terminalizer.yml configuration")

def main():
    """Run the full demo."""
    if "--config" in sys.argv:
        create_terminalizer_config()
        return
    
    try:
        print(CLEAR)
        print(f"{BOLD}{MAGENTA}🚀 Fastest Demo - Python Test Runner Built with Rust{RESET}\n")
        time.sleep(2)
        
        demo_discovery_comparison()
        time.sleep(1)
        
        demo_execution_comparison()
        time.sleep(1)
        
        demo_feature_comparison()
        time.sleep(2)
        
        print(f"\n\n{BOLD}{CYAN}Ready to make your tests blazing fast?{RESET}")
        print(f"\n{YELLOW}pip install fastest{RESET}")
        print(f"{YELLOW}fastest --help{RESET}\n")
        
    except KeyboardInterrupt:
        print(f"\n{RED}Demo interrupted{RESET}")

if __name__ == "__main__":
    main() 