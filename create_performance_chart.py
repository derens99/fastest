#!/usr/bin/env python3
"""
Create performance comparison charts
"""
import matplotlib.pyplot as plt
import numpy as np

# Data from our benchmarks
test_suites = ['Simple Tests\n(10 tests)', 'Django-Style\n(12 tests)', 'Scale Tests\n(100 tests)']
pytest_times = [170, 204, 182]  # milliseconds
fastest_simple_times = [66, 72, 78]  # milliseconds
fastest_lightning_times = [113, 99, 582]  # milliseconds

# Create figure with subplots
fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

# Bar chart comparison
x = np.arange(len(test_suites))
width = 0.25

bars1 = ax1.bar(x - width, pytest_times, width, label='pytest', color='#ff6b6b')
bars2 = ax1.bar(x, fastest_simple_times, width, label='fastest (simple)', color='#4ecdc4')
bars3 = ax1.bar(x + width, fastest_lightning_times, width, label='fastest (lightning)', color='#45b7d1')

ax1.set_xlabel('Test Suite')
ax1.set_ylabel('Execution Time (ms)')
ax1.set_title('Test Execution Time Comparison')
ax1.set_xticks(x)
ax1.set_xticklabels(test_suites)
ax1.legend()
ax1.grid(axis='y', alpha=0.3)

# Add value labels on bars
def autolabel(ax, bars):
    for bar in bars:
        height = bar.get_height()
        ax.annotate(f'{height:.0f}ms',
                    xy=(bar.get_x() + bar.get_width() / 2, height),
                    xytext=(0, 3),
                    textcoords="offset points",
                    ha='center', va='bottom',
                    fontsize=8)

autolabel(ax1, bars1)
autolabel(ax1, bars2)
autolabel(ax1, bars3)

# Speedup chart
speedup_simple = [p/f for p, f in zip(pytest_times, fastest_simple_times)]
speedup_lightning = [p/f for p, f in zip(pytest_times, fastest_lightning_times)]

bars4 = ax2.bar(x - width/2, speedup_simple, width, label='fastest (simple)', color='#4ecdc4')
bars5 = ax2.bar(x + width/2, speedup_lightning, width, label='fastest (lightning)', color='#45b7d1')

ax2.axhline(y=1, color='#ff6b6b', linestyle='--', alpha=0.5, label='pytest baseline')
ax2.set_xlabel('Test Suite')
ax2.set_ylabel('Speedup Factor')
ax2.set_title('Speedup vs pytest')
ax2.set_xticks(x)
ax2.set_xticklabels(test_suites)
ax2.legend()
ax2.grid(axis='y', alpha=0.3)

# Add speedup labels
def autolabel_speedup(ax, bars):
    for bar in bars:
        height = bar.get_height()
        ax.annotate(f'{height:.1f}x',
                    xy=(bar.get_x() + bar.get_width() / 2, height),
                    xytext=(0, 3),
                    textcoords="offset points",
                    ha='center', va='bottom',
                    fontsize=8)

autolabel_speedup(ax2, bars4)
autolabel_speedup(ax2, bars5)

plt.tight_layout()
plt.savefig('performance_comparison.png', dpi=150, bbox_inches='tight')
print("✅ Created performance_comparison.png")

# Create a summary statistics chart
fig2, ax3 = plt.subplots(figsize=(10, 6))

# Average performance across all test suites
tools = ['pytest', 'fastest\n(simple)', 'fastest\n(lightning)']
avg_times = [
    np.mean(pytest_times),
    np.mean(fastest_simple_times),
    np.mean(fastest_lightning_times)
]

colors = ['#ff6b6b', '#4ecdc4', '#45b7d1']
bars = ax3.bar(tools, avg_times, color=colors, alpha=0.8)

ax3.set_ylabel('Average Execution Time (ms)')
ax3.set_title('Average Performance Across All Test Suites')
ax3.grid(axis='y', alpha=0.3)

# Add value labels
for bar, time in zip(bars, avg_times):
    height = bar.get_height()
    ax3.annotate(f'{time:.0f}ms',
                xy=(bar.get_x() + bar.get_width() / 2, height),
                xytext=(0, 3),
                textcoords="offset points",
                ha='center', va='bottom',
                fontsize=12, fontweight='bold')

# Add speedup annotation
speedup = avg_times[0] / avg_times[1]
ax3.text(0.5, 0.95, f'fastest (simple) is {speedup:.1f}x faster than pytest on average',
         transform=ax3.transAxes, ha='center', va='top',
         fontsize=14, fontweight='bold', color='#4ecdc4',
         bbox=dict(boxstyle='round,pad=0.5', facecolor='white', alpha=0.8))

plt.tight_layout()
plt.savefig('performance_summary.png', dpi=150, bbox_inches='tight')
print("✅ Created performance_summary.png")

print("\nPerformance Summary:")
print(f"Average speedup (simple): {np.mean(speedup_simple):.2f}x")
print(f"Average speedup (lightning): {np.mean(speedup_lightning):.2f}x")
print(f"Best speedup achieved: {max(speedup_simple):.2f}x")