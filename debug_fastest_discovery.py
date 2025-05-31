#!/usr/bin/env python3
"""
Debug script to see exactly what fastest discovers using the same discovery logic
"""

import sys
sys.path.append('python/fastest_runner')

from pathlib import Path

def simulate_fastest_discovery():
    """Simulate fastest's discovery logic"""
    test_file = Path("tests/compatibility/test_real_world_patterns.py")
    
    print(f"🔍 Analyzing {test_file}")
    
    with open(test_file) as f:
        content = f.read()
    
    # Check if file has potential tests (same logic as fastest)
    has_tests = (
        "def test_" in content or
        "async def test_" in content or
        "class Test" in content or
        ("@pytest.mark" in content and "def " in content)
    )
    
    print(f"Has potential tests: {has_tests}")
    
    if not has_tests:
        return []
    
    # Now we would use tree-sitter, but let's do a simple regex analysis
    import re
    
    tests_found = []
    
    # Find all classes and their methods
    current_class = None
    lines = content.split('\n')
    
    for i, line in enumerate(lines):
        line = line.strip()
        
        # Class definition
        class_match = re.match(r'class (Test\w+)', line)
        if class_match:
            current_class = class_match.group(1)
            print(f"Found class: {current_class}")
            continue
        
        # Function definition
        func_match = re.match(r'(async\s+)?def (test_\w+)\(', line)
        if func_match:
            is_async = bool(func_match.group(1))
            func_name = func_match.group(2)
            
            # Look for decorators above this function
            decorators = []
            j = i - 1
            while j >= 0 and (lines[j].strip().startswith('@') or lines[j].strip() == ''):
                if lines[j].strip().startswith('@'):
                    decorators.append(lines[j].strip())
                j -= 1
            
            # Count parametrize cases
            param_cases = 1
            for decorator in decorators:
                if 'parametrize' in decorator:
                    # Simple counting - count commas in the parameter list
                    if '[' in decorator and ']' in decorator:
                        param_part = decorator[decorator.find('['):decorator.find(']')+1]
                        # Count top-level commas
                        depth = 0
                        comma_count = 0
                        for char in param_part:
                            if char in '([{':
                                depth += 1
                            elif char in ')]}':
                                depth -= 1
                            elif char == ',' and depth == 1:  # Top level in the list
                                comma_count += 1
                        param_cases = comma_count + 1
            
            for case in range(param_cases):
                if param_cases > 1:
                    test_id = f"{func_name}[{case}]"
                else:
                    test_id = func_name
                
                if current_class:
                    full_id = f"{test_file}::{current_class}::{test_id}"
                else:
                    full_id = f"{test_file}::{test_id}"
                
                tests_found.append({
                    'id': full_id,
                    'name': test_id,
                    'function': func_name,
                    'class': current_class,
                    'is_async': is_async,
                    'decorators': decorators,
                    'line': i + 1
                })
        
        # Reset class on unindented non-empty lines (crude but works for this analysis)
        if line and not line.startswith(' ') and not line.startswith('@') and not line.startswith('class'):
            if not line.startswith('def ') and not line.startswith('async def'):
                current_class = None
    
    return tests_found

def main():
    print("🔍 DEBUG: Simulating fastest's discovery logic")
    print("=" * 50)
    
    tests = simulate_fastest_discovery()
    
    print(f"\n📋 Found {len(tests)} tests:")
    for test in tests:
        print(f"  {test['name']} ({'async' if test['is_async'] else 'sync'}) in {test['class'] or 'module'}")
    
    print(f"\n📊 Summary:")
    print(f"Total tests: {len(tests)}")
    
    # Group by class
    classes = {}
    module_tests = []
    
    for test in tests:
        if test['class']:
            if test['class'] not in classes:
                classes[test['class']] = []
            classes[test['class']].append(test)
        else:
            module_tests.append(test)
    
    print(f"Classes: {len(classes)}")
    for class_name, class_tests in classes.items():
        print(f"  {class_name}: {len(class_tests)} tests")
    
    print(f"Module-level tests: {len(module_tests)}")

if __name__ == "__main__":
    main()