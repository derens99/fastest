#!/usr/bin/env python3
"""
Performance Regression Tracking System

Tracks performance changes over time to catch regressions early
and validate improvements during development.
"""

import json
import os
import time
import subprocess
import argparse
from pathlib import Path
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Tuple
from dataclasses import dataclass, asdict
import matplotlib.pyplot as plt
import pandas as pd
from statistics import mean, median, stdev


@dataclass
class PerformanceMetrics:
    """Performance metrics for a single run"""
    timestamp: str
    git_commit: str
    git_branch: str
    test_count: int
    discovery_time: float
    execution_time: float
    total_time: float
    memory_usage_mb: float
    cpu_usage_percent: float
    test_directory: str
    binary_version: str


@dataclass
class RegressionAlert:
    """Regression alert data"""
    metric_name: str
    current_value: float
    baseline_value: float
    change_percent: float
    severity: str  # "minor", "major", "critical"
    threshold_violated: str


class PerformanceTracker:
    """Main performance tracking system"""
    
    def __init__(self, data_dir: str = "performance_data"):
        self.data_dir = Path(data_dir)
        self.data_dir.mkdir(exist_ok=True)
        self.metrics_file = self.data_dir / "metrics.jsonl"
        self.config_file = self.data_dir / "config.json"
        self.load_config()
    
    def load_config(self):
        """Load tracker configuration"""
        default_config = {
            "regression_thresholds": {
                "discovery_time": {"minor": 10, "major": 25, "critical": 50},  # % increase
                "execution_time": {"minor": 10, "major": 25, "critical": 50},
                "total_time": {"minor": 10, "major": 25, "critical": 50},
                "memory_usage_mb": {"minor": 15, "major": 30, "critical": 60}
            },
            "baseline_window_days": 7,  # Days to look back for baseline
            "minimum_samples": 3,  # Minimum samples for stable baseline
            "alert_cooldown_hours": 24  # Hours between alerts for same metric
        }
        
        if self.config_file.exists():
            with open(self.config_file, 'r') as f:
                self.config = {**default_config, **json.load(f)}
        else:
            self.config = default_config
            self.save_config()
    
    def save_config(self):
        """Save configuration"""
        with open(self.config_file, 'w') as f:
            json.dump(self.config, f, indent=2)
    
    def get_git_info(self) -> Tuple[str, str]:
        """Get current git commit and branch"""
        try:
            commit = subprocess.check_output(
                ["git", "rev-parse", "HEAD"], 
                text=True
            ).strip()
            branch = subprocess.check_output(
                ["git", "rev-parse", "--abbrev-ref", "HEAD"], 
                text=True
            ).strip()
            return commit, branch
        except subprocess.CalledProcessError:
            return "unknown", "unknown"
    
    def get_binary_version(self, binary_path: str) -> str:
        """Get version of fastest binary"""
        try:
            result = subprocess.run(
                [binary_path, "--version"],
                capture_output=True,
                text=True,
                timeout=10
            )
            return result.stdout.strip() if result.returncode == 0 else "unknown"
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
            return "unknown"
    
    def measure_performance(self, binary_path: str, test_directory: str, 
                          runs: int = 3) -> PerformanceMetrics:
        """Measure performance metrics"""
        print(f"📊 Measuring performance over {runs} runs...")
        
        discovery_times = []
        execution_times = []
        total_times = []
        
        for i in range(runs):
            print(f"  Run {i+1}/{runs}...")
            
            # Measure discovery
            start_time = time.time()
            discovery_result = subprocess.run(
                [binary_path, "--dry-run", test_directory],
                capture_output=True,
                text=True
            )
            discovery_time = time.time() - start_time
            discovery_times.append(discovery_time)
            
            # Parse test count from discovery
            test_count = 0
            for line in discovery_result.stdout.split('\n'):
                if "collected" in line.lower() and "test" in line.lower():
                    try:
                        test_count = int([w for w in line.split() if w.isdigit()][0])
                        break
                    except (IndexError, ValueError):
                        pass
            
            # Measure execution
            start_time = time.time()
            execution_result = subprocess.run(
                [binary_path, test_directory],
                capture_output=True,
                text=True
            )
            execution_time = time.time() - start_time
            execution_times.append(execution_time)
            
            total_times.append(discovery_time + execution_time)
        
        # Get git info
        commit, branch = self.get_git_info()
        version = self.get_binary_version(binary_path)
        
        # Calculate averages
        return PerformanceMetrics(
            timestamp=datetime.now().isoformat(),
            git_commit=commit,
            git_branch=branch,
            test_count=test_count,
            discovery_time=mean(discovery_times),
            execution_time=mean(execution_times),
            total_time=mean(total_times),
            memory_usage_mb=0.0,  # TODO: Implement memory measurement
            cpu_usage_percent=0.0,  # TODO: Implement CPU measurement
            test_directory=test_directory,
            binary_version=version
        )
    
    def save_metrics(self, metrics: PerformanceMetrics):
        """Save metrics to file"""
        with open(self.metrics_file, 'a') as f:
            f.write(json.dumps(asdict(metrics)) + '\n')
        
        print(f"💾 Metrics saved: {metrics.total_time:.3f}s total")
    
    def load_metrics(self, days_back: Optional[int] = None) -> List[PerformanceMetrics]:
        """Load metrics from file"""
        if not self.metrics_file.exists():
            return []
        
        metrics = []
        cutoff_date = None
        if days_back:
            cutoff_date = datetime.now() - timedelta(days=days_back)
        
        with open(self.metrics_file, 'r') as f:
            for line in f:
                if line.strip():
                    data = json.loads(line)
                    metric_time = datetime.fromisoformat(data['timestamp'])
                    
                    if cutoff_date is None or metric_time >= cutoff_date:
                        metrics.append(PerformanceMetrics(**data))
        
        return sorted(metrics, key=lambda m: m.timestamp)
    
    def get_baseline_metrics(self, current_branch: str) -> Optional[Dict[str, float]]:
        """Get baseline metrics for comparison"""
        metrics = self.load_metrics(days_back=self.config["baseline_window_days"])
        
        # Filter to same branch
        branch_metrics = [m for m in metrics if m.git_branch == current_branch]
        
        if len(branch_metrics) < self.config["minimum_samples"]:
            print(f"⚠️  Insufficient baseline data ({len(branch_metrics)} samples)")
            return None
        
        # Calculate baseline as median of recent measurements
        return {
            "discovery_time": median([m.discovery_time for m in branch_metrics]),
            "execution_time": median([m.execution_time for m in branch_metrics]),
            "total_time": median([m.total_time for m in branch_metrics]),
            "memory_usage_mb": median([m.memory_usage_mb for m in branch_metrics])
        }
    
    def check_regressions(self, metrics: PerformanceMetrics) -> List[RegressionAlert]:
        """Check for performance regressions"""
        baseline = self.get_baseline_metrics(metrics.git_branch)
        if not baseline:
            return []
        
        alerts = []
        current_values = {
            "discovery_time": metrics.discovery_time,
            "execution_time": metrics.execution_time,
            "total_time": metrics.total_time,
            "memory_usage_mb": metrics.memory_usage_mb
        }
        
        for metric_name, current_value in current_values.items():
            baseline_value = baseline[metric_name]
            if baseline_value <= 0:
                continue
            
            change_percent = ((current_value - baseline_value) / baseline_value) * 100
            
            # Check thresholds
            thresholds = self.config["regression_thresholds"][metric_name]
            severity = None
            
            if change_percent >= thresholds["critical"]:
                severity = "critical"
            elif change_percent >= thresholds["major"]:
                severity = "major"
            elif change_percent >= thresholds["minor"]:
                severity = "minor"
            
            if severity:
                alerts.append(RegressionAlert(
                    metric_name=metric_name,
                    current_value=current_value,
                    baseline_value=baseline_value,
                    change_percent=change_percent,
                    severity=severity,
                    threshold_violated=f"{thresholds[severity]}%"
                ))
        
        return alerts
    
    def print_alerts(self, alerts: List[RegressionAlert]):
        """Print regression alerts"""
        if not alerts:
            print("✅ No performance regressions detected")
            return
        
        print(f"🚨 {len(alerts)} PERFORMANCE REGRESSION(S) DETECTED:")
        for alert in alerts:
            severity_emoji = {"minor": "⚠️", "major": "🔥", "critical": "💀"}
            print(f"  {severity_emoji[alert.severity]} {alert.metric_name}:")
            print(f"    Current: {alert.current_value:.3f}")
            print(f"    Baseline: {alert.baseline_value:.3f}")
            print(f"    Change: +{alert.change_percent:.1f}% (threshold: {alert.threshold_violated})")
    
    def generate_report(self, days_back: int = 30) -> str:
        """Generate performance report"""
        metrics = self.load_metrics(days_back=days_back)
        if not metrics:
            return "No performance data available"
        
        # Group by branch
        branches = {}
        for m in metrics:
            if m.git_branch not in branches:
                branches[m.git_branch] = []
            branches[m.git_branch].append(m)
        
        report = ["📊 PERFORMANCE REPORT", "=" * 50]
        
        for branch, branch_metrics in branches.items():
            report.append(f"\n🌿 Branch: {branch}")
            report.append(f"   Samples: {len(branch_metrics)}")
            
            if len(branch_metrics) >= 2:
                recent = branch_metrics[-5:]  # Last 5 measurements
                
                avg_discovery = mean([m.discovery_time for m in recent])
                avg_execution = mean([m.execution_time for m in recent])
                avg_total = mean([m.total_time for m in recent])
                
                report.append(f"   Average Discovery: {avg_discovery:.3f}s")
                report.append(f"   Average Execution: {avg_execution:.3f}s")
                report.append(f"   Average Total: {avg_total:.3f}s")
                
                # Trend analysis
                if len(branch_metrics) >= 3:
                    old_avg = mean([m.total_time for m in branch_metrics[:3]])
                    new_avg = mean([m.total_time for m in branch_metrics[-3:]])
                    trend = ((new_avg - old_avg) / old_avg) * 100
                    
                    trend_emoji = "📈" if trend > 5 else "📉" if trend < -5 else "➡️"
                    report.append(f"   Trend: {trend_emoji} {trend:+.1f}%")
        
        return "\n".join(report)
    
    def create_visualizations(self, days_back: int = 30):
        """Create performance visualization charts"""
        metrics = self.load_metrics(days_back=days_back)
        if len(metrics) < 2:
            print("⚠️  Insufficient data for visualizations")
            return
        
        # Convert to DataFrame
        df = pd.DataFrame([asdict(m) for m in metrics])
        df['timestamp'] = pd.to_datetime(df['timestamp'])
        
        # Create subplots
        fig, axes = plt.subplots(2, 2, figsize=(15, 10))
        fig.suptitle('Fastest Performance Metrics Over Time', fontsize=16)
        
        # Discovery time
        axes[0, 0].plot(df['timestamp'], df['discovery_time'], 'b-o', markersize=4)
        axes[0, 0].set_title('Discovery Time')
        axes[0, 0].set_ylabel('Time (seconds)')
        axes[0, 0].tick_params(axis='x', rotation=45)
        
        # Execution time
        axes[0, 1].plot(df['timestamp'], df['execution_time'], 'g-o', markersize=4)
        axes[0, 1].set_title('Execution Time')
        axes[0, 1].set_ylabel('Time (seconds)')
        axes[0, 1].tick_params(axis='x', rotation=45)
        
        # Total time
        axes[1, 0].plot(df['timestamp'], df['total_time'], 'r-o', markersize=4)
        axes[1, 0].set_title('Total Time')
        axes[1, 0].set_ylabel('Time (seconds)')
        axes[1, 0].tick_params(axis='x', rotation=45)
        
        # Test count
        axes[1, 1].plot(df['timestamp'], df['test_count'], 'm-o', markersize=4)
        axes[1, 1].set_title('Test Count')
        axes[1, 1].set_ylabel('Number of Tests')
        axes[1, 1].tick_params(axis='x', rotation=45)
        
        plt.tight_layout()
        
        # Save chart
        chart_path = self.data_dir / "performance_chart.png"
        plt.savefig(chart_path, dpi=300, bbox_inches='tight')
        print(f"📈 Chart saved: {chart_path}")
        
        plt.close()


def main():
    parser = argparse.ArgumentParser(description="Track performance regressions for fastest")
    parser.add_argument("--binary", default="fastest", help="Path to fastest binary")
    parser.add_argument("--test-dir", default="tests/", help="Test directory to benchmark")
    parser.add_argument("--runs", type=int, default=3, help="Number of benchmark runs")
    parser.add_argument("--data-dir", default="performance_data", help="Data directory")
    parser.add_argument("--report", action="store_true", help="Generate report")
    parser.add_argument("--chart", action="store_true", help="Create performance charts")
    parser.add_argument("--check-only", action="store_true", help="Only check for regressions")
    parser.add_argument("--days-back", type=int, default=30, help="Days of history for reports")
    
    args = parser.parse_args()
    
    tracker = PerformanceTracker(args.data_dir)
    
    try:
        if args.report:
            print(tracker.generate_report(args.days_back))
            return
        
        if args.chart:
            try:
                tracker.create_visualizations(args.days_back)
            except ImportError:
                print("⚠️  matplotlib/pandas not available for charts")
            return
        
        if not Path(args.test_dir).exists():
            print(f"❌ Test directory not found: {args.test_dir}")
            return 1
        
        # Measure current performance
        if not args.check_only:
            print(f"🏃 Running performance measurement...")
            metrics = tracker.measure_performance(args.binary, args.test_dir, args.runs)
            tracker.save_metrics(metrics)
        else:
            # Load latest metrics for regression check
            recent_metrics = tracker.load_metrics(days_back=1)
            if not recent_metrics:
                print("❌ No recent metrics found for regression check")
                return 1
            metrics = recent_metrics[-1]
        
        # Check for regressions
        alerts = tracker.check_regressions(metrics)
        tracker.print_alerts(alerts)
        
        # Return non-zero exit code if critical regressions found
        critical_alerts = [a for a in alerts if a.severity == "critical"]
        if critical_alerts:
            return 2
        
        major_alerts = [a for a in alerts if a.severity == "major"]
        if major_alerts:
            return 1
        
        return 0
        
    except KeyboardInterrupt:
        print("\n👋 Performance tracking stopped")
        return 0
    except Exception as e:
        print(f"❌ Error: {e}")
        return 1


if __name__ == "__main__":
    exit(main())