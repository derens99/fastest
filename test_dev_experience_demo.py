#!/usr/bin/env python3
"""
Demonstration of the Developer Experience features integrated into Fastest
This shows how the new DevExperienceManager enhances debugging and error reporting
"""

def test_basic_assertion():
    """A simple test that should pass"""
    assert 2 + 2 == 4

def test_failing_assertion():
    """A test that fails to demonstrate enhanced error reporting"""
    actual = 2 + 2
    expected = 5
    assert actual == expected, f"Expected {expected}, but got {actual}"

def test_attribute_error():
    """A test that has an AttributeError to show error suggestions"""
    obj = {}
    obj.non_existent_method()

def test_with_parameters():
    """A parametrized test (would need @pytest.mark.parametrize in real usage)"""
    values = [1, 2, 3]
    for value in values:
        assert value > 0

if __name__ == "__main__":
    print("🚀 Developer Experience Demo for Fastest Python Test Runner")
    print("=" * 60)
    print()
    print("This file demonstrates the enhanced developer features:")
    print("  ✓ Enhanced error reporting with colored output")
    print("  ✓ Debugging support with --pdb flag")
    print("  ✓ IDE integration for test discovery")
    print("  ✓ Timeout handling for long-running tests")
    print("  ✓ Contextual error suggestions")
    print()
    print("Features integrated:")
    print("  • DevExperienceManager in executor/ultra_fast.rs")
    print("  • Enhanced error reporting with suggestions")
    print("  • Debug mode with Python pdb integration")
    print("  • IDE metadata export for test discovery")
    print("  • Professional error output with colors")
    print()
    print("Usage examples:")
    print("  cargo run --bin fastest -- test_dev_experience_demo.py")
    print("  cargo run --bin fastest -- test_dev_experience_demo.py --pdb")
    print("  cargo run --bin fastest -- test_dev_experience_demo.py --enhanced-errors")
    print()
    print("The enhanced developer experience is now ready! 🎉")