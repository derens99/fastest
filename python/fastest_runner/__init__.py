#!/usr/bin/env python3
"""
⚡ Fastest - The blazing fast Python test runner
"""

__version__ = "0.1.0"

import os
import sys
import shutil


def main(argv=None):
    """Main entry point for the fastest CLI."""
    
    if argv is None:
        argv = sys.argv[1:]
    
    # Try to find fastest binary in PATH first, but exclude the current script
    current_script = os.path.realpath(sys.argv[0])
    fastest_binary = None
    
    # Check PATH for fastest binary (excluding pip-installed wrapper)
    path_binary = shutil.which("fastest")
    if path_binary and os.path.realpath(path_binary) != current_script:
        fastest_binary = path_binary
    
    if not fastest_binary:
        # Try common installation locations
        possible_paths = [
            os.path.expanduser("~/.local/bin/fastest"),
            "/usr/local/bin/fastest", 
            "/opt/homebrew/bin/fastest",
            # Also try in project directory for development
            os.path.join(os.path.dirname(__file__), "..", "..", "target", "release", "fastest"),
        ]
        
        for path in possible_paths:
            abs_path = os.path.abspath(path)
            if os.path.isfile(abs_path) and os.access(abs_path, os.X_OK):
                fastest_binary = abs_path
                break
    
    if not fastest_binary:
        print("❌ Error: fastest binary not found!", file=sys.stderr)
        print("", file=sys.stderr) 
        print("To install fastest, run:", file=sys.stderr)
        print("  curl -LsSf https://raw.githubusercontent.com/derens99/fastest/main/install.sh | sh", file=sys.stderr)
        return 1
    
    # Execute the fastest binary with the provided arguments
    try:
        os.execvp(fastest_binary, [fastest_binary] + argv)
    except Exception as e:
        print(f"❌ Error executing fastest: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())