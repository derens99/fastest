#!/usr/bin/env python3
"""
Development Dashboard for Fastest

Real-time development feedback showing:
- Performance vs pytest comparison
- Compatibility test results  
- Regression tracking
- Feature completeness status
"""

import json
import time
import subprocess
import os
import sys
from pathlib import Path
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Tuple
import argparse


class DevelopmentDashboard:
    """Main dashboard for development feedback"""
    
    def __init__(self, fastest_binary: str = "fastest"):
        self.fastest_binary = fastest_binary
        self.project_root = Path.cwd()
        self.results_dir = self.project_root / "comparison_results"
        self.perf_data_dir = self.project_root / "performance_data"
        
        # Test directories
        self.compatibility_tests = self.project_root / "tests" / "compatibility"
        self.sample_tests = self.project_root / "tests" / "test_sample.py"
        
    def get_git_status(self) -> Dict[str, str]:
        """Get current git status"""
        try:
            branch = subprocess.check_output(
                ["git", "rev-parse", "--abbrev-ref", "HEAD"], 
                text=True
            ).strip()
            
            commit = subprocess.check_output(
                ["git", "rev-parse", "--short", "HEAD"], 
                text=True
            ).strip()
            
            # Check if working directory is clean
            status_output = subprocess.check_output(
                ["git", "status", "--porcelain"], 
                text=True
            )
            
            clean = len(status_output.strip()) == 0
            
            return {
                "branch": branch,
                "commit": commit,
                "clean": clean,
                "status": "clean" if clean else "modified"
            }
        except subprocess.CalledProcessError:
            return {
                "branch": "unknown",
                "commit": "unknown", 
                "clean": False,
                "status": "error"
            }
    
    def check_binary_availability(self) -> Dict[str, any]:
        """Check if binaries are available"""
        def check_binary(name: str) -> Dict[str, any]:
            try:
                result = subprocess.run(
                    [name, "--version"],
                    capture_output=True,
                    text=True,
                    timeout=5
                )
                return {
                    "available": result.returncode == 0,
                    "version": result.stdout.strip() if result.returncode == 0 else "unknown",
                    "error": result.stderr.strip() if result.returncode != 0 else None
                }
            except (subprocess.CalledProcessError, subprocess.TimeoutExpired, FileNotFoundError):
                return {"available": False, "version": "unknown", "error": "not found"}
        
        return {
            "fastest": check_binary(self.fastest_binary),
            "pytest": check_binary("pytest"),
            "python": check_binary("python")
        }
    
    def run_quick_comparison(self) -> Dict[str, any]:
        """Run quick performance comparison"""
        if not self.compatibility_tests.exists():
            return {"error": "Compatibility tests not found"}
        
        try:
            # Run comparison script
            comparison_script = self.project_root / "scripts" / "compare_with_pytest.py"
            if not comparison_script.exists():
                return {"error": "Comparison script not found"}
            
            result = subprocess.run(
                ["python", str(comparison_script), str(self.compatibility_tests), "--quiet"],
                capture_output=True,
                text=True,
                timeout=60
            )
            
            if result.returncode == 0:
                # Load latest results
                latest_file = self.results_dir / "latest.json"
                if latest_file.exists():
                    with open(latest_file, 'r') as f:
                        data = json.load(f)
                    return {
                        "success": True,
                        "discovery_speedup": data.get("performance_ratio", {}).get("discovery_speedup", 1.0),
                        "execution_speedup": data.get("performance_ratio", {}).get("execution_speedup", 1.0),
                        "compatibility_score": data.get("compatibility_analysis", {}).get("compatibility_score", 0.0),
                        "fastest_tests": data.get("fastest_execution", {}).get("total", 0),
                        "pytest_tests": data.get("pytest_execution", {}).get("total", 0)
                    }
            
            return {"error": f"Comparison failed: {result.stderr}"}
            
        except Exception as e:
            return {"error": f"Comparison error: {e}"}
    
    def get_recent_performance_data(self) -> Dict[str, any]:
        """Get recent performance metrics"""
        metrics_file = self.perf_data_dir / "metrics.jsonl"
        if not metrics_file.exists():
            return {"error": "No performance data found"}
        
        try:
            # Read last few lines
            with open(metrics_file, 'r') as f:
                lines = f.readlines()
            
            if not lines:
                return {"error": "No performance data"}
            
            # Parse recent metrics
            recent_metrics = []
            for line in lines[-5:]:  # Last 5 measurements
                if line.strip():
                    recent_metrics.append(json.loads(line))
            
            if not recent_metrics:
                return {"error": "No valid performance data"}
            
            latest = recent_metrics[-1]
            
            # Calculate trend if we have multiple points
            trend = "stable"
            if len(recent_metrics) >= 2:
                old_time = recent_metrics[0]["total_time"]
                new_time = latest["total_time"]
                change = ((new_time - old_time) / old_time) * 100
                
                if change > 10:
                    trend = "slower"
                elif change < -10:
                    trend = "faster"
            
            return {
                "success": True,
                "latest_total_time": latest["total_time"],
                "latest_discovery_time": latest["discovery_time"],
                "latest_execution_time": latest["execution_time"],
                "test_count": latest["test_count"],
                "trend": trend,
                "samples": len(recent_metrics)
            }
            
        except Exception as e:
            return {"error": f"Performance data error: {e}"}
    
    def check_test_compatibility(self) -> Dict[str, any]:
        """Check compatibility test results"""
        if not self.compatibility_tests.exists():
            return {"error": "Compatibility tests not found"}
        
        try:
            # Run compatibility tests with fastest
            result = subprocess.run(
                [self.fastest_binary, str(self.compatibility_tests)],
                capture_output=True,
                text=True,
                timeout=30
            )
            
            # Parse results
            output = result.stdout + result.stderr
            passed = failed = skipped = 0
            
            for line in output.split('\n'):
                line = line.lower()
                if "passed" in line or "failed" in line or "skipped" in line:
                    words = line.split()
                    for i, word in enumerate(words):
                        if word.isdigit():
                            count = int(word)
                            if i + 1 < len(words):
                                status = words[i + 1]
                                if status.startswith("passed"):
                                    passed = count
                                elif status.startswith("failed"):
                                    failed = count
                                elif status.startswith("skipped"):
                                    skipped = count
            
            total = passed + failed + skipped
            success_rate = (passed / total * 100) if total > 0 else 0
            
            return {
                "success": True,
                "passed": passed,
                "failed": failed,
                "skipped": skipped,
                "total": total,
                "success_rate": success_rate,
                "exit_code": result.returncode
            }
            
        except Exception as e:
            return {"error": f"Compatibility test error: {e}"}
    
    def get_feature_completeness(self) -> Dict[str, any]:
        """Assess feature completeness status"""
        # This could be enhanced to check actual implementation
        # For now, provide static assessment based on known status
        
        features = {
            "Core Test Discovery": {"status": "complete", "score": 100},
            "Test Execution": {"status": "complete", "score": 95},
            "Parallel Execution": {"status": "complete", "score": 90},
            "Basic Fixtures": {"status": "complete", "score": 85},
            "Parametrized Tests": {"status": "complete", "score": 90},
            "Async Tests": {"status": "complete", "score": 85},
            "Basic Markers": {"status": "complete", "score": 80},
            "Session Fixtures": {"status": "partial", "score": 40},
            "Plugin System": {"status": "partial", "score": 60},
            "Coverage Integration": {"status": "missing", "score": 0},
            "Pytest Plugin Compat": {"status": "missing", "score": 20},
            "IDE Integration": {"status": "missing", "score": 0}
        }
        
        total_score = sum(f["score"] for f in features.values())
        max_score = len(features) * 100
        overall_completeness = (total_score / max_score) * 100
        
        return {
            "features": features,
            "overall_completeness": overall_completeness,
            "complete_features": len([f for f in features.values() if f["status"] == "complete"]),
            "partial_features": len([f for f in features.values() if f["status"] == "partial"]),
            "missing_features": len([f for f in features.values() if f["status"] == "missing"])
        }
    
    def print_dashboard(self):
        """Print the development dashboard"""
        print("\n" + "="*80)
        print("🚀 FASTEST DEVELOPMENT DASHBOARD")
        print("="*80)
        
        # Git status
        git_info = self.get_git_status()
        status_emoji = "✅" if git_info["clean"] else "⚠️"
        print(f"📝 Git Status: {status_emoji} {git_info['branch']} @ {git_info['commit']} ({git_info['status']})")
        
        # Binary availability
        print("\n🔧 BINARY STATUS:")
        binaries = self.check_binary_availability()
        for name, info in binaries.items():
            status = "✅" if info["available"] else "❌"
            print(f"  {status} {name}: {info['version']}")
        
        # Quick comparison
        print("\n⚡ PERFORMANCE COMPARISON:")
        comparison = self.run_quick_comparison()
        if "error" in comparison:
            print(f"  ❌ {comparison['error']}")
        else:
            print(f"  🔍 Discovery: {comparison['discovery_speedup']:.1f}x faster than pytest")
            print(f"  ⚡ Execution: {comparison['execution_speedup']:.1f}x faster than pytest")
            print(f"  🎯 Compatibility: {comparison['compatibility_score']:.1%}")
            print(f"  📊 Tests: {comparison['fastest_tests']} found")
        
        # Performance trends
        print("\n📈 PERFORMANCE TRENDS:")
        perf_data = self.get_recent_performance_data()
        if "error" in perf_data:
            print(f"  ❌ {perf_data['error']}")
        else:
            trend_emoji = {"faster": "📈", "slower": "📉", "stable": "➡️"}
            print(f"  ⏱️  Latest: {perf_data['latest_total_time']:.3f}s total")
            print(f"  🔍 Discovery: {perf_data['latest_discovery_time']:.3f}s")
            print(f"  ⚡ Execution: {perf_data['latest_execution_time']:.3f}s")
            print(f"  📊 Trend: {trend_emoji[perf_data['trend']]} {perf_data['trend']} ({perf_data['samples']} samples)")
        
        # Compatibility tests
        print("\n🧪 COMPATIBILITY TESTS:")
        compat = self.check_test_compatibility()
        if "error" in compat:
            print(f"  ❌ {compat['error']}")
        else:
            success_emoji = "✅" if compat["success_rate"] > 90 else "⚠️" if compat["success_rate"] > 70 else "❌"
            print(f"  {success_emoji} Success Rate: {compat['success_rate']:.1f}%")
            print(f"  📊 Results: {compat['passed']} passed, {compat['failed']} failed, {compat['skipped']} skipped")
        
        # Feature completeness
        print("\n🎯 FEATURE COMPLETENESS:")
        features = self.get_feature_completeness()
        print(f"  📊 Overall: {features['overall_completeness']:.1f}% complete")
        print(f"  ✅ Complete: {features['complete_features']} features")
        print(f"  🚧 Partial: {features['partial_features']} features")
        print(f"  ❌ Missing: {features['missing_features']} features")
        
        # Top priorities
        print("\n🎯 TOP PRIORITIES:")
        partial_missing = [
            name for name, info in features["features"].items() 
            if info["status"] in ["partial", "missing"]
        ]
        for i, feature in enumerate(partial_missing[:3], 1):
            status = features["features"][feature]["status"]
            emoji = "🚧" if status == "partial" else "❌"
            print(f"  {i}. {emoji} {feature}")
        
        print("\n" + "="*80)
        print(f"🕒 Last updated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
        print("="*80)
    
    def watch_mode(self, interval: int = 30):
        """Run dashboard in watch mode"""
        print("👀 Starting watch mode... (Ctrl+C to stop)")
        try:
            while True:
                # Clear screen
                os.system('clear' if os.name == 'posix' else 'cls')
                self.print_dashboard()
                print(f"\n⏳ Refreshing in {interval} seconds...")
                time.sleep(interval)
        except KeyboardInterrupt:
            print("\n👋 Dashboard stopped")


def main():
    parser = argparse.ArgumentParser(description="Development dashboard for fastest")
    parser.add_argument("--fastest-binary", default="fastest", help="Path to fastest binary")
    parser.add_argument("--watch", action="store_true", help="Run in watch mode")
    parser.add_argument("--interval", type=int, default=30, help="Watch mode refresh interval")
    parser.add_argument("--json", action="store_true", help="Output as JSON")
    
    args = parser.parse_args()
    
    dashboard = DevelopmentDashboard(args.fastest_binary)
    
    try:
        if args.watch:
            dashboard.watch_mode(args.interval)
        else:
            if args.json:
                # JSON output for CI/automation
                data = {
                    "git": dashboard.get_git_status(),
                    "binaries": dashboard.check_binary_availability(),
                    "comparison": dashboard.run_quick_comparison(),
                    "performance": dashboard.get_recent_performance_data(),
                    "compatibility": dashboard.check_test_compatibility(),
                    "features": dashboard.get_feature_completeness(),
                    "timestamp": datetime.now().isoformat()
                }
                print(json.dumps(data, indent=2))
            else:
                dashboard.print_dashboard()
        
        return 0
        
    except KeyboardInterrupt:
        print("\n👋 Dashboard stopped")
        return 0
    except Exception as e:
        print(f"❌ Dashboard error: {e}")
        return 1


if __name__ == "__main__":
    exit(main())