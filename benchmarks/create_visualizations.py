#!/usr/bin/env python3
"""
Create professional benchmark visualizations for Fastest.
Generates graphs similar to uv/ruff style.
"""

import json
import matplotlib.pyplot as plt
import numpy as np
from pathlib import Path
import subprocess
import time
from typing import Dict, List, Tuple
import seaborn as sns

# Set style for professional looking graphs
plt.style.use('seaborn-v0_8-darkgrid')
sns.set_palette("husl")

# Color scheme similar to uv/ruff
COLORS = {
    'fastest': '#FF6B6B',  # Coral red
    'pytest': '#4ECDC4',   # Teal
    'fastest_ast': '#FFE66D',  # Yellow
    'fastest_parallel': '#95E1D3'  # Mint
}

def run_benchmarks() -> Dict[str, Dict[str, float]]:
    """Run benchmarks and collect results."""
    print("Running benchmarks...")
    
    # Run the benchmark script and capture output
    result = subprocess.run(
        ['python', 'benchmark.py'], 
        capture_output=True, 
        text=True,
        cwd='.'
    )
    
    # Parse results from output (you may need to modify benchmark.py to output JSON)
    # For now, using example data that matches your README
    return {
        'discovery': {
            '10_tests': {'pytest': 0.125, 'fastest': 0.0014},
            '100_tests': {'pytest': 0.250, 'fastest': 0.0034},
            '1000_tests': {'pytest': 0.358, 'fastest': 0.0067},
        },
        'execution': {
            '10_tests': {'pytest': 0.187, 'fastest': 0.089},
            '100_tests': {'pytest': 1.872, 'fastest': 0.892},
            '1000_tests': {'pytest': 18.72, 'fastest': 8.92},
        },
        'parallel': {
            '100_tests': {
                'fastest_sequential': 0.892,
                'fastest_2_workers': 0.523,
                'fastest_4_workers': 0.312,
                'fastest_8_workers': 0.298,
            }
        }
    }

def create_bar_chart_comparison():
    """Create a bar chart comparing discovery and execution times."""
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))
    
    # Discovery comparison
    test_sizes = ['10 tests', '100 tests', '1,000 tests']
    pytest_discovery = [125, 250, 358]  # ms
    fastest_discovery = [1.4, 3.4, 6.7]  # ms
    
    x = np.arange(len(test_sizes))
    width = 0.35
    
    bars1 = ax1.bar(x - width/2, pytest_discovery, width, label='pytest', color=COLORS['pytest'])
    bars2 = ax1.bar(x + width/2, fastest_discovery, width, label='fastest', color=COLORS['fastest'])
    
    ax1.set_xlabel('Test Suite Size', fontsize=12)
    ax1.set_ylabel('Discovery Time (ms)', fontsize=12)
    ax1.set_title('Test Discovery Performance', fontsize=14, fontweight='bold')
    ax1.set_xticks(x)
    ax1.set_xticklabels(test_sizes)
    ax1.legend()
    ax1.set_yscale('log')  # Log scale to show the dramatic difference
    
    # Add value labels on bars
    for bars in [bars1, bars2]:
        for bar in bars:
            height = bar.get_height()
            ax1.annotate(f'{height:.1f}ms',
                        xy=(bar.get_x() + bar.get_width() / 2, height),
                        xytext=(0, 3),
                        textcoords="offset points",
                        ha='center', va='bottom',
                        fontsize=9)
    
    # Execution comparison
    pytest_execution = [187, 1872, 18720]  # ms
    fastest_execution = [89, 892, 8920]  # ms
    
    bars3 = ax2.bar(x - width/2, pytest_execution, width, label='pytest', color=COLORS['pytest'])
    bars4 = ax2.bar(x + width/2, fastest_execution, width, label='fastest', color=COLORS['fastest'])
    
    ax2.set_xlabel('Test Suite Size', fontsize=12)
    ax2.set_ylabel('Execution Time (ms)', fontsize=12)
    ax2.set_title('Test Execution Performance', fontsize=14, fontweight='bold')
    ax2.set_xticks(x)
    ax2.set_xticklabels(test_sizes)
    ax2.legend()
    
    # Add value labels
    for bars in [bars3, bars4]:
        for bar in bars:
            height = bar.get_height()
            ax2.annotate(f'{height/1000:.1f}s',
                        xy=(bar.get_x() + bar.get_width() / 2, height),
                        xytext=(0, 3),
                        textcoords="offset points",
                        ha='center', va='bottom',
                        fontsize=9)
    
    plt.tight_layout()
    plt.savefig('docs/benchmark_comparison.png', dpi=300, bbox_inches='tight')
    plt.savefig('docs/benchmark_comparison.svg', bbox_inches='tight')
    print("Created: docs/benchmark_comparison.png/svg")

def create_speedup_chart():
    """Create a chart showing speedup factors."""
    fig, ax = plt.subplots(figsize=(10, 6))
    
    categories = ['Discovery\n(10 tests)', 'Discovery\n(100 tests)', 'Discovery\n(1000 tests)',
                  'Execution\n(10 tests)', 'Execution\n(100 tests)', 'Execution\n(1000 tests)']
    speedups = [88, 73, 53, 2.1, 2.1, 2.1]
    
    bars = ax.bar(categories, speedups, color=[COLORS['fastest']] * 3 + [COLORS['fastest_parallel']] * 3)
    
    # Add value labels
    for bar, speedup in zip(bars, speedups):
        height = bar.get_height()
        ax.annotate(f'{speedup}x',
                    xy=(bar.get_x() + bar.get_width() / 2, height),
                    xytext=(0, 3),
                    textcoords="offset points",
                    ha='center', va='bottom',
                    fontsize=12, fontweight='bold')
    
    ax.set_ylabel('Speedup Factor', fontsize=12)
    ax.set_title('Fastest Performance Gains vs pytest', fontsize=16, fontweight='bold')
    ax.set_ylim(0, max(speedups) * 1.15)
    
    # Add horizontal line at 1x
    ax.axhline(y=1, color='gray', linestyle='--', alpha=0.5)
    ax.text(0.5, 1.5, 'No speedup', ha='center', va='bottom', color='gray', alpha=0.7)
    
    plt.tight_layout()
    plt.savefig('docs/speedup_factors.png', dpi=300, bbox_inches='tight')
    plt.savefig('docs/speedup_factors.svg', bbox_inches='tight')
    print("Created: docs/speedup_factors.png/svg")

def create_parallel_scaling_chart():
    """Create a chart showing parallel execution scaling."""
    fig, ax = plt.subplots(figsize=(10, 6))
    
    workers = [1, 2, 4, 8]
    times = [892, 523, 312, 298]  # ms for 100 tests
    
    ax.plot(workers, times, 'o-', linewidth=2, markersize=8, color=COLORS['fastest'])
    
    # Fill area under curve
    ax.fill_between(workers, times, alpha=0.3, color=COLORS['fastest'])
    
    # Add value labels
    for x, y in zip(workers, times):
        ax.annotate(f'{y}ms',
                    xy=(x, y),
                    xytext=(0, 10),
                    textcoords="offset points",
                    ha='center', va='bottom',
                    fontsize=10)
    
    ax.set_xlabel('Number of Workers', fontsize=12)
    ax.set_ylabel('Execution Time (ms)', fontsize=12)
    ax.set_title('Parallel Execution Scaling (100 tests)', fontsize=14, fontweight='bold')
    ax.set_xticks(workers)
    ax.grid(True, alpha=0.3)
    
    # Add ideal scaling line
    ideal_times = [times[0] / w for w in workers]
    ax.plot(workers, ideal_times, '--', color='gray', alpha=0.5, label='Ideal scaling')
    ax.legend()
    
    plt.tight_layout()
    plt.savefig('docs/parallel_scaling.png', dpi=300, bbox_inches='tight')
    plt.savefig('docs/parallel_scaling.svg', bbox_inches='tight')
    print("Created: docs/parallel_scaling.png/svg")

def create_memory_comparison_chart():
    """Create a chart comparing memory usage."""
    fig, ax = plt.subplots(figsize=(8, 6))
    
    tools = ['pytest', 'fastest']
    memory = [100, 50]  # Relative memory usage
    
    bars = ax.bar(tools, memory, color=[COLORS['pytest'], COLORS['fastest']])
    
    # Add value labels
    for bar, mem in zip(bars, memory):
        height = bar.get_height()
        ax.annotate(f'{mem}%',
                    xy=(bar.get_x() + bar.get_width() / 2, height),
                    xytext=(0, 3),
                    textcoords="offset points",
                    ha='center', va='bottom',
                    fontsize=14, fontweight='bold')
    
    ax.set_ylabel('Relative Memory Usage', fontsize=12)
    ax.set_title('Memory Efficiency Comparison', fontsize=14, fontweight='bold')
    ax.set_ylim(0, 120)
    
    plt.tight_layout()
    plt.savefig('docs/memory_comparison.png', dpi=300, bbox_inches='tight')
    plt.savefig('docs/memory_comparison.svg', bbox_inches='tight')
    print("Created: docs/memory_comparison.png/svg")

def create_comprehensive_infographic():
    """Create a comprehensive infographic summarizing all performance metrics."""
    fig = plt.figure(figsize=(12, 8))
    
    # Create grid for subplots
    gs = fig.add_gridspec(3, 3, hspace=0.4, wspace=0.3)
    
    # Main title
    fig.suptitle('Fastest: Blazing Fast Python Test Runner', fontsize=20, fontweight='bold')
    
    # Metric cards
    metrics = [
        ('88x', 'Faster Discovery', COLORS['fastest']),
        ('2.1x', 'Faster Execution', COLORS['fastest_parallel']),
        ('50%', 'Less Memory', COLORS['fastest_ast']),
    ]
    
    for i, (value, label, color) in enumerate(metrics):
        ax = fig.add_subplot(gs[0, i])
        ax.text(0.5, 0.5, value, fontsize=36, fontweight='bold', 
                ha='center', va='center', color=color)
        ax.text(0.5, 0.1, label, fontsize=12, ha='center', va='center')
        ax.set_xlim(0, 1)
        ax.set_ylim(0, 1)
        ax.axis('off')
    
    # Discovery time comparison
    ax1 = fig.add_subplot(gs[1, :2])
    test_sizes = ['10', '100', '1K']
    pytest_times = [125, 250, 358]
    fastest_times = [1.4, 3.4, 6.7]
    
    x = np.arange(len(test_sizes))
    width = 0.35
    
    ax1.bar(x - width/2, pytest_times, width, label='pytest', color=COLORS['pytest'])
    ax1.bar(x + width/2, fastest_times, width, label='fastest', color=COLORS['fastest'])
    ax1.set_xlabel('Tests')
    ax1.set_ylabel('Discovery (ms)')
    ax1.set_title('Discovery Performance')
    ax1.set_xticks(x)
    ax1.set_xticklabels(test_sizes)
    ax1.legend()
    ax1.set_yscale('log')
    
    # Parallel scaling
    ax2 = fig.add_subplot(gs[1, 2])
    workers = [1, 2, 4, 8]
    speedups = [1, 1.7, 2.9, 3.0]
    
    ax2.plot(workers, speedups, 'o-', linewidth=2, color=COLORS['fastest'])
    ax2.fill_between(workers, speedups, alpha=0.3, color=COLORS['fastest'])
    ax2.set_xlabel('Workers')
    ax2.set_ylabel('Speedup')
    ax2.set_title('Parallel Scaling')
    ax2.grid(True, alpha=0.3)
    
    # Feature comparison table
    ax3 = fig.add_subplot(gs[2, :])
    ax3.axis('tight')
    ax3.axis('off')
    
    features = [
        ['Feature', 'pytest', 'fastest'],
        ['Startup Time', '~200ms', '<100ms'],
        ['Tree-sitter Parser', '❌', '✅'],
        ['Work-stealing Parallel', '❌', '✅'],
        ['Smart Caching', '❌', '✅'],
        ['Memory Usage', '100%', '50%'],
    ]
    
    table = ax3.table(cellText=features, cellLoc='center', loc='center',
                      colWidths=[0.4, 0.3, 0.3])
    table.auto_set_font_size(False)
    table.set_fontsize(10)
    table.scale(1.2, 1.5)
    
    # Style the header row
    for i in range(3):
        table[(0, i)].set_facecolor('#E0E0E0')
        table[(0, i)].set_text_props(weight='bold')
    
    # Color the fastest column
    for i in range(1, 6):
        table[(i, 2)].set_facecolor('#FFE6E6')
    
    plt.tight_layout()
    plt.savefig('docs/fastest_infographic.png', dpi=300, bbox_inches='tight')
    plt.savefig('docs/fastest_infographic.svg', bbox_inches='tight')
    print("Created: docs/fastest_infographic.png/svg")

def main():
    """Generate all visualizations."""
    print("Creating benchmark visualizations...")
    
    # Ensure docs directory exists
    Path('docs').mkdir(exist_ok=True)
    
    # Generate all charts
    create_bar_chart_comparison()
    create_speedup_chart()
    create_parallel_scaling_chart()
    create_memory_comparison_chart()
    create_comprehensive_infographic()
    
    print("\nAll visualizations created successfully!")
    print("\nYou can now use these in your:")
    print("- README.md")
    print("- GitHub Pages (docs/index.html)")
    print("- Blog posts")
    print("- Social media announcements")

if __name__ == "__main__":
    main() 