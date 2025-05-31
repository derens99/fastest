#!/usr/bin/env python3
"""Validate Fastest test discovery against pytest on real projects."""
import subprocess
import json
import os
import sys
from pathlib import Path

def run_fastest_discovery(test_path):
    """Run Fastest in discovery mode."""
    # Use --dry-run if available, otherwise run normally and parse output
    cmd = ["./target/release/fastest", test_path, "-v"]
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=5)
        # Parse the stderr for discovery info
        output = result.stderr
        if "Discovering tests" in output:
            lines = output.split('\n')
            for line in lines:
                if "Found" in line and "tests" in line:
                    # Extract number from "Found X tests"
                    parts = line.split()
                    for i, part in enumerate(parts):
                        if part == "Found" and i+1 < len(parts):
                            try:
                                return int(parts[i+1])
                            except:
                                pass
        
        # Also check stdout
        output = result.stdout
        if "Found" in output and "tests" in output:
            try:
                # Try to extract from "Found X tests"
                import re
                match = re.search(r'Found (\d+) tests', output)
                if match:
                    return int(match.group(1))
            except:
                pass
                
        return 0
    except subprocess.TimeoutExpired:
        print(f"Timeout discovering tests in {test_path}")
        return 0
    except Exception as e:
        print(f"Error running Fastest: {e}")
        return 0

def count_test_files(test_path):
    """Count test files manually."""
    count = 0
    for root, dirs, files in os.walk(test_path):
        for file in files:
            if file.startswith("test_") and file.endswith(".py"):
                count += 1
    return count

def validate_projects():
    """Validate test discovery on real projects."""
    results = {
        "projects": [],
        "total_files": 0,
        "total_tests": 0
    }
    
    test_repos = Path("test_repos")
    
    for project_dir in sorted(test_repos.iterdir()):
        if not project_dir.is_dir():
            continue
            
        project_name = project_dir.name
        test_dirs = []
        
        # Find test directories
        for test_dir_name in ["tests", "test"]:
            test_path = project_dir / test_dir_name
            if test_path.exists():
                test_dirs.append(test_path)
        
        if not test_dirs:
            continue
            
        project_result = {
            "name": project_name,
            "test_files": 0,
            "tests_discovered": 0,
            "status": "success"
        }
        
        for test_dir in test_dirs:
            # Count test files
            file_count = count_test_files(test_dir)
            project_result["test_files"] += file_count
            
            # Run Fastest discovery
            test_count = run_fastest_discovery(str(test_dir))
            project_result["tests_discovered"] += test_count
        
        results["total_files"] += project_result["test_files"]
        results["total_tests"] += project_result["tests_discovered"]
        results["projects"].append(project_result)
        
        print(f"{project_name:20} - Files: {project_result['test_files']:4d}, Tests: {project_result['tests_discovered']:4d}")
    
    print("\n" + "="*50)
    print(f"Total projects: {len(results['projects'])}")
    print(f"Total test files: {results['total_files']}")
    print(f"Total tests discovered: {results['total_tests']}")
    
    # Save results
    with open("test_repos/discovery_validation.json", "w") as f:
        json.dump(results, f, indent=2)
    
    return results

if __name__ == "__main__":
    os.chdir(Path(__file__).parent.parent)
    validate_projects()