"""
Marker system compatible with pytest.mark
"""

class MarkDecorator:
    """A decorator for marking tests."""
    
    def __init__(self, name, args=None, kwargs=None):
        self.name = name
        self.args = args or ()
        self.kwargs = kwargs or {}
    
    def __call__(self, func):
        """Apply marker to function."""
        # Add marker to function's attributes
        if not hasattr(func, '_markers'):
            func._markers = []
        func._markers.append(self)
        return func
    
    def __repr__(self):
        return f"<MarkDecorator {self.name}>"


class MarkGenerator:
    """Generate markers with any name."""
    
    def __getattr__(self, name):
        """Create a marker with the given name."""
        def marker_factory(*args, **kwargs):
            return MarkDecorator(name, args, kwargs)
        
        # Make it callable without arguments too
        marker_factory = MarkDecorator(name)
        marker_factory.__name__ = name
        return marker_factory


# Create the main mark instance
mark = MarkGenerator()

# Pre-define common markers for better IDE support
mark.skip = lambda reason=None: MarkDecorator("skip", kwargs={"reason": reason} if reason else {})
mark.skipif = lambda condition, *, reason=None: MarkDecorator("skipif", args=(condition,), kwargs={"reason": reason} if reason else {})
mark.xfail = lambda condition=None, *, reason=None, raises=None, strict=False: MarkDecorator("xfail", args=(condition,) if condition else (), kwargs={"reason": reason, "raises": raises, "strict": strict})
mark.parametrize = lambda argnames, argvalues, indirect=False, ids=None: MarkDecorator("parametrize", args=(argnames, argvalues), kwargs={"indirect": indirect, "ids": ids})
mark.slow = MarkDecorator("slow")
mark.integration = MarkDecorator("integration")
mark.unit = MarkDecorator("unit") 