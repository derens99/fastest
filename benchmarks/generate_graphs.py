#!/usr/bin/env python3
"""
Generate comparison graphs from benchmark results.

Reads benchmarks/results/results.json and produces:
  - benchmarks/results/discovery_speed.png     -- discovery/collection comparison
  - benchmarks/results/execution_speed.png     -- parallel execution comparison
  - benchmarks/results/speedup_chart.png       -- speedup multiplier chart

Usage:
    python benchmarks/generate_graphs.py
"""

import json
import sys
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import matplotlib.ticker as ticker
import numpy as np

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------
REPO_ROOT = Path(__file__).resolve().parent.parent
RESULTS_DIR = REPO_ROOT / "benchmarks" / "results"
DATA_FILE = RESULTS_DIR / "results.json"

# ---------------------------------------------------------------------------
# Style
# ---------------------------------------------------------------------------
COLOR_FASTEST = "#FF6B35"   # vibrant orange
COLOR_PYTEST  = "#4A90D9"   # steel blue
COLOR_SPEEDUP = "#2ECC71"   # emerald green
COLOR_SPEEDUP_LOW = "#F39C12"  # amber for < 2x
BG_COLOR = "#FAFAFA"

plt.rcParams.update({
    "font.family": "sans-serif",
    "font.sans-serif": ["Segoe UI", "Helvetica Neue", "Arial", "sans-serif"],
    "font.size": 11,
    "axes.titlesize": 15,
    "axes.titleweight": "bold",
    "axes.labelsize": 12,
    "figure.facecolor": BG_COLOR,
    "axes.facecolor": "#FFFFFF",
    "axes.edgecolor": "#CCCCCC",
    "axes.grid": True,
    "grid.color": "#E8E8E8",
    "grid.linewidth": 0.5,
    "grid.alpha": 0.7,
})


def load_data():
    with open(DATA_FILE) as f:
        return json.load(f)


# ---------------------------------------------------------------------------
# Chart 1: Discovery speed
# ---------------------------------------------------------------------------
def make_discovery_chart(data):
    disc = data["discovery"]
    n_files = [r["n_files"] for r in disc]
    total_tests = [r["total_tests"] for r in disc]
    fastest_ms = [r["fastest"]["mean"] * 1000 for r in disc]  # convert to ms
    pytest_ms  = [r["pytest"]["mean"] * 1000 for r in disc]

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 5.5))

    # Left: bar chart
    x = np.arange(len(n_files))
    width = 0.35
    bars1 = ax1.bar(x - width/2, fastest_ms, width, label="Fastest",
                    color=COLOR_FASTEST, edgecolor="white", linewidth=0.8, zorder=3)
    bars2 = ax1.bar(x + width/2, pytest_ms, width, label="pytest",
                    color=COLOR_PYTEST, edgecolor="white", linewidth=0.8, zorder=3)

    # Value labels
    for bar in bars1:
        h = bar.get_height()
        ax1.text(bar.get_x() + bar.get_width()/2, h + 15,
                 f"{h:.0f}ms", ha="center", va="bottom", fontsize=8,
                 fontweight="bold", color=COLOR_FASTEST)
    for bar in bars2:
        h = bar.get_height()
        ax1.text(bar.get_x() + bar.get_width()/2, h + 15,
                 f"{h:.0f}ms", ha="center", va="bottom", fontsize=8,
                 fontweight="bold", color=COLOR_PYTEST)

    ax1.set_xlabel("Number of Test Files")
    ax1.set_ylabel("Collection Time (ms)")
    ax1.set_title("Test Discovery Speed")
    ax1.set_xticks(x)
    labels = [f"{f} files\n({t} tests)" for f, t in zip(n_files, total_tests)]
    ax1.set_xticklabels(labels, fontsize=9)
    ax1.legend(loc="upper left", frameon=True, fancybox=True)
    ax1.set_ylim(bottom=0)

    # Right: speedup
    speedups = [r["speedup"] for r in disc]
    bars = ax2.bar([f"{f} files" for f in n_files], speedups,
                   color=COLOR_SPEEDUP, edgecolor="white", linewidth=0.8, zorder=3, width=0.55)
    for bar, sp in zip(bars, speedups):
        ax2.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 0.3,
                 f"{sp:.0f}x", ha="center", va="bottom", fontsize=12,
                 fontweight="bold", color="#1A1A1A")

    ax2.set_xlabel("Number of Test Files")
    ax2.set_ylabel("Speedup (x faster)")
    ax2.set_title("Discovery Speedup Over pytest")
    ax2.set_ylim(bottom=0, top=max(speedups) * 1.2)
    ax2.axhline(y=1, color="#E74C3C", linestyle="--", linewidth=1, alpha=0.5)

    fig.suptitle("Test Discovery: Fastest vs pytest", fontsize=17, fontweight="bold", y=1.01)
    fig.tight_layout()
    path = RESULTS_DIR / "discovery_speed.png"
    fig.savefig(path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"  [OK] {path}")


# ---------------------------------------------------------------------------
# Chart 2: Realistic execution speed
# ---------------------------------------------------------------------------
def make_execution_chart(data):
    real = data["realistic"]
    sizes = [r["size"] for r in real]
    fastest_t = [r["fastest"]["mean"] for r in real]
    pytest_t  = [r["pytest"]["mean"] for r in real]

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 5.5))

    # Left: line chart showing scaling
    ax1.plot(sizes, fastest_t, "o-", color=COLOR_FASTEST, linewidth=2.5,
             markersize=9, markeredgecolor="white", markeredgewidth=1.5,
             label="Fastest (parallel)", zorder=4)
    ax1.plot(sizes, pytest_t, "s-", color=COLOR_PYTEST, linewidth=2.5,
             markersize=9, markeredgecolor="white", markeredgewidth=1.5,
             label="pytest (sequential)", zorder=4)

    # Fill the gap
    ax1.fill_between(sizes, fastest_t, pytest_t,
                     where=[p > f for p, f in zip(pytest_t, fastest_t)],
                     alpha=0.10, color=COLOR_SPEEDUP, zorder=2,
                     label="Time saved")

    # Annotate crossover region
    for i, (s, ft, pt) in enumerate(zip(sizes, fastest_t, pytest_t)):
        if pt > ft and (i == 0 or pytest_t[i-1] <= fastest_t[i-1]):
            ax1.annotate("Parallelism\nwins here",
                        xy=(s, (ft + pt) / 2),
                        xytext=(s - 80, (ft + pt) / 2 + 1.5),
                        fontsize=9, fontweight="bold", color=COLOR_SPEEDUP,
                        arrowprops=dict(arrowstyle="->", color=COLOR_SPEEDUP, lw=1.5))
            break

    ax1.set_xlabel("Number of Tests")
    ax1.set_ylabel("Wall-Clock Time (seconds)")
    ax1.set_title("Execution Time (tests with ~10ms work)")
    ax1.legend(loc="upper left", frameon=True, fancybox=True)
    ax1.set_ylim(bottom=0)

    # Right: speedup bars
    speedups = [r["speedup"] for r in real]
    colors = [COLOR_SPEEDUP if sp >= 1.5 else COLOR_SPEEDUP_LOW if sp >= 1 else "#E74C3C" for sp in speedups]
    bars = ax2.bar([str(s) for s in sizes], speedups,
                   color=colors, edgecolor="white", linewidth=0.8, zorder=3, width=0.55)
    for bar, sp in zip(bars, speedups):
        label = f"{sp:.1f}x"
        ax2.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 0.05,
                 label, ha="center", va="bottom", fontsize=11,
                 fontweight="bold", color="#1A1A1A")

    ax2.axhline(y=1, color="#E74C3C", linestyle="--", linewidth=1, alpha=0.5, label="1x (same speed)")
    ax2.set_xlabel("Number of Tests")
    ax2.set_ylabel("Speedup (x faster)")
    ax2.set_title("Execution Speedup Over pytest")
    ax2.set_ylim(bottom=0, top=max(speedups) * 1.25)
    ax2.legend(loc="upper left", frameon=True, fancybox=True)

    fig.suptitle("Parallel Execution: Fastest vs pytest", fontsize=17, fontweight="bold", y=1.01)
    fig.tight_layout()
    path = RESULTS_DIR / "execution_speed.png"
    fig.savefig(path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"  [OK] {path}")


# ---------------------------------------------------------------------------
# Chart 3: Combined hero image for README
# ---------------------------------------------------------------------------
def make_hero_chart(data):
    disc = data["discovery"]
    real = data["realistic"]

    fig = plt.figure(figsize=(16, 10))
    gs = fig.add_gridspec(2, 2, hspace=0.38, wspace=0.30)

    # ---- Top-left: Discovery bar chart ----
    ax1 = fig.add_subplot(gs[0, 0])
    n_files = [r["n_files"] for r in disc]
    total_tests = [r["total_tests"] for r in disc]
    fastest_ms = [r["fastest"]["mean"] * 1000 for r in disc]
    pytest_ms  = [r["pytest"]["mean"] * 1000 for r in disc]
    x = np.arange(len(n_files))
    width = 0.35
    ax1.bar(x - width/2, fastest_ms, width, label="Fastest", color=COLOR_FASTEST,
            edgecolor="white", linewidth=0.8, zorder=3)
    ax1.bar(x + width/2, pytest_ms, width, label="pytest", color=COLOR_PYTEST,
            edgecolor="white", linewidth=0.8, zorder=3)
    ax1.set_xlabel("Test Files")
    ax1.set_ylabel("Time (ms)")
    ax1.set_title("Test Discovery")
    ax1.set_xticks(x)
    ax1.set_xticklabels([f"{f}" for f in n_files], fontsize=9)
    ax1.legend(fontsize=9, loc="upper left")
    ax1.set_ylim(bottom=0)

    # ---- Top-right: Discovery speedup ----
    ax2 = fig.add_subplot(gs[0, 1])
    disc_speedups = [r["speedup"] for r in disc]
    bars = ax2.bar([str(f) for f in n_files], disc_speedups,
                   color=COLOR_SPEEDUP, edgecolor="white", linewidth=0.8, zorder=3, width=0.55)
    for bar, sp in zip(bars, disc_speedups):
        ax2.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 0.3,
                 f"{sp:.0f}x", ha="center", va="bottom", fontsize=10,
                 fontweight="bold", color="#1A1A1A")
    ax2.set_xlabel("Test Files")
    ax2.set_ylabel("Speedup")
    ax2.set_title("Discovery Speedup")
    ax2.set_ylim(bottom=0, top=max(disc_speedups) * 1.2)

    # ---- Bottom-left: Execution scaling ----
    ax3 = fig.add_subplot(gs[1, 0])
    sizes = [r["size"] for r in real]
    fastest_t = [r["fastest"]["mean"] for r in real]
    pytest_t  = [r["pytest"]["mean"] for r in real]
    ax3.plot(sizes, fastest_t, "o-", color=COLOR_FASTEST, linewidth=2.5,
             markersize=8, markeredgecolor="white", markeredgewidth=1.5,
             label="Fastest (parallel)", zorder=4)
    ax3.plot(sizes, pytest_t, "s-", color=COLOR_PYTEST, linewidth=2.5,
             markersize=8, markeredgecolor="white", markeredgewidth=1.5,
             label="pytest (sequential)", zorder=4)
    ax3.fill_between(sizes, fastest_t, pytest_t,
                     where=[p > f for p, f in zip(pytest_t, fastest_t)],
                     alpha=0.10, color=COLOR_SPEEDUP, zorder=2)
    ax3.set_xlabel("Number of Tests")
    ax3.set_ylabel("Time (seconds)")
    ax3.set_title("Execution Time (~10ms/test)")
    ax3.legend(fontsize=9, loc="upper left")
    ax3.set_ylim(bottom=0)

    # ---- Bottom-right: Execution speedup ----
    ax4 = fig.add_subplot(gs[1, 1])
    real_speedups = [r["speedup"] for r in real]
    colors = [COLOR_SPEEDUP if sp >= 1.5 else COLOR_SPEEDUP_LOW if sp >= 1 else "#E74C3C"
              for sp in real_speedups]
    bars = ax4.bar([str(s) for s in sizes], real_speedups,
                   color=colors, edgecolor="white", linewidth=0.8, zorder=3, width=0.55)
    for bar, sp in zip(bars, real_speedups):
        ax4.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 0.05,
                 f"{sp:.1f}x", ha="center", va="bottom", fontsize=10,
                 fontweight="bold", color="#1A1A1A")
    ax4.axhline(y=1, color="#E74C3C", linestyle="--", linewidth=1, alpha=0.5)
    ax4.set_xlabel("Number of Tests")
    ax4.set_ylabel("Speedup")
    ax4.set_title("Execution Speedup")
    ax4.set_ylim(bottom=0, top=max(real_speedups) * 1.25)

    fig.suptitle("Fastest Performance Benchmarks", fontsize=19, fontweight="bold")
    path = RESULTS_DIR / "benchmark_combined.png"
    fig.savefig(path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"  [OK] {path}")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
if __name__ == "__main__":
    if not DATA_FILE.exists():
        sys.exit(f"ERROR: {DATA_FILE} not found.\nRun `python benchmarks/run_benchmarks.py` first.")

    data = load_data()
    print("Generating graphs...")
    make_discovery_chart(data)
    make_execution_chart(data)
    make_hero_chart(data)
    print(f"\nDone! All graphs saved to {RESULTS_DIR}/")
