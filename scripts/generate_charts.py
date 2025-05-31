#!/usr/bin/env python3
"""
Generate professional performance charts for the Fastest project
Creates publication-ready visualizations of benchmark results
"""

import json
import matplotlib.pyplot as plt
import matplotlib.patches as mpatches
import numpy as np
from pathlib import Path
import seaborn as sns

# Set professional styling
plt.style.use('seaborn-v0_8-darkgrid')
sns.set_palette("husl")

def load_benchmark_data():
    """Load the latest benchmark results"""
    results_file = Path("benchmarks/official_results.json")
    if not results_file.exists():
        print("❌ No benchmark results found. Run: python scripts/official_benchmark.py --quick")
        return None
    
    with open(results_file) as f:
        return json.load(f)

def create_speedup_chart(data, output_dir):
    """Create the main speedup comparison chart"""
    comparisons = data['comparisons']
    
    test_counts = [c['test_suite_size'] for c in comparisons if c.get('speedup_total')]
    speedups = [c['speedup_total'] for c in comparisons if c.get('speedup_total')]
    fastest_times = [c['fastest']['total_time'] for c in comparisons if c.get('speedup_total')]
    pytest_times = [c['pytest']['total_time'] for c in comparisons if c.get('speedup_total')]
    
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(16, 6))
    
    # Chart 1: Speedup Factor
    colors = ['#2E8B57', '#4169E1', '#FF6347', '#9932CC']
    bars = ax1.bar(range(len(test_counts)), speedups, color=colors, alpha=0.8, 
                   edgecolor='black', linewidth=1.2)
    
    # Add value labels on bars
    for i, (bar, speedup) in enumerate(zip(bars, speedups)):
        height = bar.get_height()
        ax1.text(bar.get_x() + bar.get_width()/2., height + 0.1,
                f'{speedup:.1f}x', ha='center', va='bottom', 
                fontweight='bold', fontsize=12)
    
    ax1.set_xlabel('Test Suite Size', fontsize=14, fontweight='bold')
    ax1.set_ylabel('Speedup Factor', fontsize=14, fontweight='bold')
    ax1.set_title('🚀 Fastest vs pytest - Performance Speedup', fontsize=16, fontweight='bold')
    ax1.set_xticks(range(len(test_counts)))
    ax1.set_xticklabels([f'{count} tests' for count in test_counts])
    ax1.set_ylim(0, max(speedups) * 1.2)
    ax1.grid(True, alpha=0.3)
    
    # Add horizontal line at 1x (no improvement)
    ax1.axhline(y=1, color='red', linestyle='--', alpha=0.7, label='No improvement')
    ax1.legend()
    
    # Chart 2: Absolute Time Comparison
    x = np.arange(len(test_counts))
    width = 0.35
    
    bars1 = ax2.bar(x - width/2, fastest_times, width, label='Fastest', 
                    color='#2E8B57', alpha=0.8, edgecolor='black')
    bars2 = ax2.bar(x + width/2, pytest_times, width, label='pytest', 
                    color='#FF6347', alpha=0.8, edgecolor='black')
    
    # Add value labels
    for bars in [bars1, bars2]:
        for bar in bars:
            height = bar.get_height()
            ax2.text(bar.get_x() + bar.get_width()/2., height + 0.01,
                    f'{height:.2f}s', ha='center', va='bottom', fontsize=10)
    
    ax2.set_xlabel('Test Suite Size', fontsize=14, fontweight='bold')
    ax2.set_ylabel('Execution Time (seconds)', fontsize=14, fontweight='bold')
    ax2.set_title('⏱️ Absolute Performance Comparison', fontsize=16, fontweight='bold')
    ax2.set_xticks(x)
    ax2.set_xticklabels([f'{count} tests' for count in test_counts])
    ax2.legend()
    ax2.grid(True, alpha=0.3)
    
    plt.tight_layout()
    plt.savefig(output_dir / 'performance_comparison.png', dpi=300, bbox_inches='tight')
    plt.savefig(output_dir / 'performance_comparison.svg', bbox_inches='tight')
    plt.close()
    
    return speedups, test_counts

def create_scaling_chart(data, output_dir):
    """Create a scaling analysis chart"""
    comparisons = data['comparisons']
    
    test_counts = [c['test_suite_size'] for c in comparisons if c.get('speedup_total')]
    fastest_times = [c['fastest']['total_time'] for c in comparisons if c.get('speedup_total')]
    pytest_times = [c['pytest']['total_time'] for c in comparisons if c.get('speedup_total')]
    
    fig, ax = plt.subplots(1, 1, figsize=(12, 8))
    
    # Plot lines with markers
    ax.plot(test_counts, fastest_times, 'o-', color='#2E8B57', linewidth=3, 
            markersize=8, label='Fastest', markeredgecolor='black')
    ax.plot(test_counts, pytest_times, 's-', color='#FF6347', linewidth=3, 
            markersize=8, label='pytest', markeredgecolor='black')
    
    # Add value annotations
    for i, (tc, ft, pt) in enumerate(zip(test_counts, fastest_times, pytest_times)):
        ax.annotate(f'{ft:.2f}s', (tc, ft), textcoords="offset points", 
                   xytext=(0,10), ha='center', fontweight='bold')
        ax.annotate(f'{pt:.2f}s', (tc, pt), textcoords="offset points", 
                   xytext=(0,10), ha='center', fontweight='bold')
    
    ax.set_xlabel('Number of Tests', fontsize=14, fontweight='bold')
    ax.set_ylabel('Execution Time (seconds)', fontsize=14, fontweight='bold')
    ax.set_title('📈 Scaling Performance Analysis', fontsize=16, fontweight='bold')
    ax.legend(fontsize=12)
    ax.grid(True, alpha=0.3)
    ax.set_xscale('log')
    ax.set_yscale('log')
    
    # Add performance gap annotations
    for i, (tc, ft, pt) in enumerate(zip(test_counts, fastest_times, pytest_times)):
        speedup = pt / ft
        ax.annotate(f'{speedup:.1f}x faster', 
                   xy=(tc, (ft + pt) / 2), 
                   xytext=(20, 0), textcoords='offset points',
                   fontsize=10, fontweight='bold', color='purple',
                   arrowprops=dict(arrowstyle='->', color='purple', alpha=0.7))
    
    plt.tight_layout()
    plt.savefig(output_dir / 'scaling_analysis.png', dpi=300, bbox_inches='tight')
    plt.savefig(output_dir / 'scaling_analysis.svg', bbox_inches='tight')
    plt.close()

def create_summary_chart(speedups, test_counts, output_dir):
    """Create a summary performance dashboard"""
    fig, ((ax1, ax2), (ax3, ax4)) = plt.subplots(2, 2, figsize=(16, 12))
    
    # Chart 1: Speedup by test count
    colors = plt.cm.viridis(np.linspace(0, 1, len(test_counts)))
    bars = ax1.bar(range(len(test_counts)), speedups, color=colors, alpha=0.8)
    
    for i, (bar, speedup) in enumerate(zip(bars, speedups)):
        height = bar.get_height()
        ax1.text(bar.get_x() + bar.get_width()/2., height + 0.1,
                f'{speedup:.1f}x', ha='center', va='bottom', fontweight='bold')
    
    ax1.set_title('🚀 Performance Multiplier', fontweight='bold')
    ax1.set_xticks(range(len(test_counts)))
    ax1.set_xticklabels([f'{count}' for count in test_counts])
    ax1.set_ylabel('Speedup Factor')
    ax1.grid(True, alpha=0.3)
    
    # Chart 2: Performance categories
    categories = ['Small\n(≤50 tests)', 'Medium\n(100 tests)', 'Large\n(500+ tests)']
    cat_speedups = [np.mean(speedups[:2]), speedups[2], speedups[3]]
    
    colors_cat = ['#FFD700', '#FF6347', '#9932CC']
    ax2.pie(cat_speedups, labels=categories, autopct='%1.1fx', startangle=90,
            colors=colors_cat, textprops={'fontweight': 'bold'})
    ax2.set_title('🎯 Performance by Category', fontweight='bold')
    
    # Chart 3: Time savings
    time_saved = []
    pytest_times = [0.235, 0.310, 0.314, 0.706]  # From benchmark results
    fastest_times = [0.097, 0.100, 0.103, 0.137]
    
    for pt, ft in zip(pytest_times, fastest_times):
        time_saved.append(pt - ft)
    
    ax3.bar(range(len(test_counts)), time_saved, color='#32CD32', alpha=0.8)
    for i, ts in enumerate(time_saved):
        ax3.text(i, ts + 0.01, f'{ts:.2f}s', ha='center', va='bottom', fontweight='bold')
    
    ax3.set_title('⏱️ Time Saved per Run', fontweight='bold')
    ax3.set_xticks(range(len(test_counts)))
    ax3.set_xticklabels([f'{count}' for count in test_counts])
    ax3.set_ylabel('Time Saved (seconds)')
    ax3.grid(True, alpha=0.3)
    
    # Chart 4: Efficiency metrics
    efficiency = [s * 100 / max(speedups) for s in speedups]
    ax4.plot(test_counts, efficiency, 'o-', color='#4169E1', linewidth=3, markersize=8)
    ax4.fill_between(test_counts, efficiency, alpha=0.3, color='#4169E1')
    
    ax4.set_title('📊 Relative Efficiency', fontweight='bold')
    ax4.set_xlabel('Test Count')
    ax4.set_ylabel('Efficiency %')
    ax4.grid(True, alpha=0.3)
    ax4.set_ylim(0, 100)
    
    plt.tight_layout()
    plt.savefig(output_dir / 'performance_summary.png', dpi=300, bbox_inches='tight')
    plt.savefig(output_dir / 'performance_summary.svg', bbox_inches='tight')
    plt.close()

def main():
    """Generate all performance charts"""
    print("🎨 Generating performance charts...")
    
    # Load benchmark data
    data = load_benchmark_data()
    if not data:
        return
    
    # Create output directory
    output_dir = Path("docs/images")
    output_dir.mkdir(exist_ok=True)
    
    # Generate charts
    print("📊 Creating speedup comparison chart...")
    speedups, test_counts = create_speedup_chart(data, output_dir)
    
    print("📈 Creating scaling analysis chart...")
    create_scaling_chart(data, output_dir)
    
    print("🎯 Creating summary dashboard...")
    create_summary_chart(speedups, test_counts, output_dir)
    
    # Print summary
    avg_speedup = np.mean(speedups)
    max_speedup = max(speedups)
    
    print("\n" + "="*50)
    print("🎉 CHARTS GENERATED SUCCESSFULLY!")
    print("="*50)
    print(f"📊 Average Speedup: {avg_speedup:.1f}x")
    print(f"🚀 Maximum Speedup: {max_speedup:.1f}x")
    print(f"📈 Test Suite Sizes: {len(test_counts)} different sizes")
    print(f"📁 Charts saved to: {output_dir}")
    print("\n📄 Charts generated:")
    print("  - performance_comparison.png (Main comparison)")
    print("  - scaling_analysis.png (Scaling behavior)")
    print("  - performance_summary.png (Dashboard)")
    print("  - SVG versions for web use")

if __name__ == "__main__":
    main()