"""
Fastest test runner marker support.

This module provides pytest-compatible markers under the `fastest.mark` namespace.
You can use these decorators just like pytest.mark:

    import fastest
    
    @fastest.mark.skip(reason="Not implemented yet")
    def test_something():
        pass
"""

import functools
import sys

class MarkDecorator:
    """A test marker decorator."""
    
    def __init__(self, name, args=None, kwargs=None):
        self.name = name
        self.args = args or ()
        self.kwargs = kwargs or {}
        self.mark_name = f"fastest.mark.{name}"
    
    def __call__(self, *call_args, **call_kwargs):
        """Handle both @mark and @mark() syntax."""
        if len(call_args) == 1 and callable(call_args[0]) and not call_kwargs:
            # Direct decoration: @mark
            func = call_args[0]
            if not hasattr(func, '_markers'):
                func._markers = []
            func._markers.append(self)
            
            # Also add as a regular attribute for compatibility
            if not hasattr(func, self.mark_name):
                setattr(func, self.mark_name, True)
            
            return func
        else:
            # Called with arguments: @mark(reason="...") 
            # Return a new MarkDecorator with the arguments
            return MarkDecorator(self.name, call_args, call_kwargs)
    
    def __repr__(self):
        args_str = ', '.join(repr(arg) for arg in self.args)
        kwargs_str = ', '.join(f"{k}={v!r}" for k, v in self.kwargs.items())
        all_args = ', '.join(filter(None, [args_str, kwargs_str]))
        
        if all_args:
            return f"fastest.mark.{self.name}({all_args})"
        return f"fastest.mark.{self.name}"

class MarkGenerator:
    """Generate test markers dynamically."""
    
    def __getattr__(self, name):
        """Create a marker decorator for any attribute access."""
        # Return a MarkDecorator instance that can be used as @mark or @mark()
        return MarkDecorator(name)

# Create the mark namespace
mark = MarkGenerator()

# Common markers as shortcuts
skip = mark.skip
xfail = mark.xfail
slow = mark.slow
asyncio = mark.asyncio

# Fixture placeholder (for future implementation)
class fixture:
    """Fixture decorator (placeholder for future implementation)."""
    
    def __init__(self, scope="function", autouse=False, params=None):
        self.scope = scope
        self.autouse = autouse
        self.params = params
    
    def __call__(self, func):
        # For now, just return the function unchanged
        # Future: Register as a fixture
        return func

# Export main API
__all__ = ['mark', 'skip', 'xfail', 'slow', 'asyncio', 'fixture'] 