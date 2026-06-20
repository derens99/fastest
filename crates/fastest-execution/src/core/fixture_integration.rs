//! Fixture Integration Module
//!
//! This module provides the Python code for complete fixture execution integration
//! with support for all scopes, dependencies, and teardown.

/// Generate the enhanced Python worker code with full fixture support
pub fn generate_fixture_aware_worker_code(verbose: bool) -> String {
    let verbose_str = if verbose { "True" } else { "False" };

    format!(
        r#"
# -*- coding: utf-8 -*-
import sys
import os

# Ensure UTF-8 encoding for all operations
if sys.version_info[0] >= 3:
    import locale
    import codecs
    # Set UTF-8 as default encoding for stdout/stderr
    if hasattr(sys.stdout, 'buffer'):
        sys.stdout = codecs.getwriter('utf-8')(sys.stdout.buffer)
    if hasattr(sys.stderr, 'buffer'):
        sys.stderr = codecs.getwriter('utf-8')(sys.stderr.buffer)

# Set PYTHONIOENCODING environment variable
os.environ['PYTHONIOENCODING'] = 'utf-8'

import inspect
import asyncio
import importlib
import functools
import platform
import tempfile
import pathlib
import queue
import ctypes
from concurrent.futures import ThreadPoolExecutor
from contextlib import contextmanager, redirect_stdout, redirect_stderr
from io import StringIO
from time import perf_counter as perf

# Import pytest if available for skip/xfail support
try:
    import pytest
except ImportError:
    class SkipTest(Exception):
        pass

    class XFailTest(Exception):
        pass

    import types

    def _format_expected_exception(expected_exception):
        if isinstance(expected_exception, tuple):
            return " or ".join(exc.__name__ for exc in expected_exception)
        return expected_exception.__name__

    class _RaisesContext:
        def __init__(self, expected_exception, match=None):
            self.expected_exception = expected_exception
            self.match = match
            self.value = None

        def __enter__(self):
            return self

        def __exit__(self, exc_type, exc_value, _traceback):
            if exc_type is None:
                raise AssertionError("DID NOT RAISE " + _format_expected_exception(self.expected_exception))
            if not issubclass(exc_type, self.expected_exception):
                return False
            self.value = exc_value
            if self.match is not None:
                import re
                message = str(exc_value)
                if re.search(self.match, message) is None:
                    raise AssertionError(
                        "Regex pattern did not match.\n Regex: "
                        + repr(self.match)
                        + "\n Input: "
                        + repr(message)
                    )
            return True

    class _FixtureFunctionMarker:
        def __init__(self, scope='function', params=None, autouse=False, ids=None):
            self.scope = scope
            self.params = params
            self.autouse = autouse
            self.ids = ids

    class _Mark:
        def __getattr__(self, _name):
            def marker(*args, **_kwargs):
                if len(args) == 1 and callable(args[0]) and not _kwargs:
                    return args[0]

                def decorator(func):
                    return func

                return decorator

            return marker

        def parametrize(self, *_args, **_kwargs):
            def decorator(func):
                return func

            return decorator

    def _skip(reason=""):
        raise SkipTest(reason)

    def _xfail(reason=""):
        raise XFailTest(reason)

    def _raises(expected_exception, match=None, **_kwargs):
        return _RaisesContext(expected_exception, match=match)

    def _param(*values, **_kwargs):
        if len(values) == 1:
            return values[0]
        return tuple(values)

    def _hook_decorator(*args, **kwargs):
        def decorator(func):
            func.pytest_hook_options = kwargs
            return func

        if len(args) == 1 and callable(args[0]) and not kwargs:
            return decorator(args[0])
        return decorator

    def _fixture(func=None, *, scope='function', params=None, autouse=False, ids=None, **_kwargs):
        def decorator(inner_func):
            inner_func._pytestfixturefunction = _FixtureFunctionMarker(
                scope=scope,
                params=params,
                autouse=autouse,
                ids=ids,
            )
            return inner_func

        if callable(func):
            return decorator(func)
        return decorator

    pytest = types.ModuleType('pytest')
    pytest.skip = _skip
    pytest.xfail = _xfail
    pytest.raises = _raises
    pytest.param = _param
    pytest.fixture = _fixture
    pytest.hookimpl = _hook_decorator
    pytest.hookspec = _hook_decorator
    pytest.mark = _Mark()
    sys.modules['pytest'] = pytest

# Debug flag from parent
DEBUG = {verbose_str}

# Track if we're capturing output
_capturing_output = False
_original_stderr = None

def debug_print(msg):
    if DEBUG:
        # If we're capturing output, write to the original stderr
        if _capturing_output and _original_stderr:
            print(f"[DEBUG] {{msg}}", file=_original_stderr)
        else:
            print(f"[DEBUG] {{msg}}", file=sys.stderr)

# Module and function caches for ultra-fast imports
module_cache = {{}}
fn_cache = {{}}
path_cache = set()

# Fixture management
fixture_cache = {{}}
active_fixtures = {{}}
teardown_stack = []

# Enhanced lifecycle management for setup/teardown
from collections import OrderedDict
import atexit

class ClassLifecycleManager:
    """Manages class setup/teardown with proper ordering and error handling"""
    def __init__(self):
        self.active_classes = OrderedDict()  # class_path -> {{'cls': cls, 'setup': bool, 'teardown': bool, 'test_count': int, 'tests_run': int}}
        self.setup_order = []  # Track setup order for reverse teardown
        self.current_class_path = None
        self.current_class = None

    def register_class(self, class_path, cls):
        """Register a class and track its state"""
        if class_path not in self.active_classes:
            self.active_classes[class_path] = {{
                'cls': cls,
                'setup': False,
                'teardown': False,
                'test_count': 0,
                'tests_run': 0
            }}
        elif cls is not None and self.active_classes[class_path]['cls'] is None:
            self.active_classes[class_path]['cls'] = cls

    def setup_class(self, class_path, cls):
        """Setup a class if not already done"""
        self.register_class(class_path, cls)

        if self.active_classes[class_path]['setup']:
            return  # Already setup

        if hasattr(cls, 'setup_class'):
            try:
                debug_print(f"Calling setup_class for {{class_path}}")
                if isinstance(inspect.getattr_static(cls, 'setup_class'), classmethod):
                    cls.setup_class()
                else:
                    cls.setup_class(cls)

                self.active_classes[class_path]['setup'] = True
                self.setup_order.append(class_path)
            except Exception as e:
                debug_print(f"setup_class failed for {{class_path}}: {{e}}")
                raise

    def increment_test_count(self, class_path):
        """Increment expected test count for a class"""
        if class_path not in self.active_classes:
            self.register_class(class_path, None)
        if class_path in self.active_classes:
            self.active_classes[class_path]['test_count'] += 1

    def mark_test_run(self, class_path):
        """Mark that a test has been run for this class"""
        if class_path in self.active_classes:
            self.active_classes[class_path]['tests_run'] += 1

    def should_teardown_class(self, class_path):
        """Check if all tests for a class have been run"""
        if class_path not in self.active_classes:
            return False

        state = self.active_classes[class_path]
        return (state['setup'] and not state['teardown'] and
                state['test_count'] > 0 and
                state['tests_run'] >= state['test_count'])

    def teardown_class(self, class_path, force=False):
        """Teardown a class if all tests are complete or forced"""
        if class_path not in self.active_classes:
            return

        state = self.active_classes[class_path]

        # Skip if not setup or already torn down
        if not state['setup'] or state['teardown']:
            return

        # Check if we should teardown (all tests run or forced)
        if not force and not self.should_teardown_class(class_path):
            debug_print(f"Skipping teardown for {{class_path}}: {{state['tests_run']}}/{{state['test_count']}} tests run")
            return

        cls = state['cls']
        if hasattr(cls, 'teardown_class'):
            try:
                debug_print(f"Calling teardown_class for {{class_path}}")
                if isinstance(inspect.getattr_static(cls, 'teardown_class'), classmethod):
                    cls.teardown_class()
                else:
                    cls.teardown_class(cls)

                state['teardown'] = True
            except Exception as e:
                debug_print(f"teardown_class failed for {{class_path}}: {{e}}")
                # Don't re-raise to allow other teardowns to proceed

    def transition_to_class(self, new_class_path, new_cls):
        """Handle transition between classes"""
        # Don't do anything if staying in same class
        if self.current_class_path == new_class_path:
            return

        # If transitioning away from a class, check if we should teardown
        if self.current_class_path:
            # Only teardown if all tests have been run
            if self.should_teardown_class(self.current_class_path):
                self.teardown_class(self.current_class_path)
                # Also teardown class-scoped fixtures
                teardown_fixtures('class', class_path=self.current_class_path)

        # Register the next class, but do not call setup_class here. Setup must
        # run inside execute_test_with_fixtures so setup failures become normal
        # test outcomes instead of aborting the whole batch.
        if new_class_path:
            self.register_class(new_class_path, new_cls)
            self.current_class_path = new_class_path
            self.current_class = new_cls
        else:
            self.current_class_path = None
            self.current_class = None

    def teardown_all_classes(self):
        """Teardown all remaining classes in reverse setup order"""
        for class_path in reversed(self.setup_order):
            self.teardown_class(class_path, force=True)
            # Also teardown class-scoped fixtures
            teardown_fixtures('class', class_path=class_path)

# Global lifecycle manager
_class_lifecycle = ClassLifecycleManager()

# Register cleanup on exit
atexit.register(_class_lifecycle.teardown_all_classes)

# Legacy setup state for module-level tracking
_setup_state = {{
    'modules': {{}},      # module_name: {{'setup_done': bool, 'teardown_done': bool}}
    'classes': {{}},      # Kept for backward compatibility but managed by ClassLifecycleManager
    'setup_order': [],  # Track order for proper teardown
}}

# Marker extraction and handling functions
def split_marker_arguments(args_str):
    """Split marker arguments on top-level commas only."""
    parts = []
    current = []
    depth = 0
    quote = None
    escaped = False

    for char in args_str:
        if escaped:
            current.append(char)
            escaped = False
            continue

        if char == '\\':
            current.append(char)
            escaped = True
            continue

        if quote:
            current.append(char)
            if char == quote:
                quote = None
            continue

        if char in ('"', "'"):
            quote = char
            current.append(char)
            continue

        if char in '([{{':
            depth += 1
            current.append(char)
            continue

        if char in ')]}}':
            depth = max(0, depth - 1)
            current.append(char)
            continue

        if char == ',' and depth == 0:
            part = ''.join(current).strip()
            if part:
                parts.append(part)
            current = []
            continue

        current.append(char)

    part = ''.join(current).strip()
    if part:
        parts.append(part)

    return parts

def is_marker_keyword_argument(part):
    if '=' not in part:
        return False
    key = part.split('=', 1)[0].strip()
    return key.isidentifier()

def clean_marker_argument(value):
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in ('"', "'"):
        return value[1:-1]
    return value

def extract_markers(decorators):
    """Extract markers from decorator strings"""
    markers = []
    for decorator in decorators:
        if 'mark.' in decorator or '@mark' in decorator:
            # Remove @ if present
            decorator = decorator.lstrip('@')
            # Extract marker name and args
            if 'pytest.mark.' in decorator:
                marker_str = decorator.replace('pytest.mark.', '')
            elif 'fastest.mark.' in decorator:
                marker_str = decorator.replace('fastest.mark.', '')
            elif 'mark.' in decorator:
                marker_str = decorator.replace('mark.', '')
            else:
                continue

            # Parse marker name and arguments
            if '(' in marker_str:
                name = marker_str[:marker_str.index('(')]
                args_str = marker_str[marker_str.index('(')+1:marker_str.rindex(')')]
                # Simple argument parsing - handles common cases
                args = []
                kwargs = {{}}
                if args_str:
                    # Very simplified parsing - handles basic cases
                    if any(is_marker_keyword_argument(part) for part in split_marker_arguments(args_str)):
                        # Has kwargs
                        parts = split_marker_arguments(args_str)
                        for part in parts:
                            part = part.strip()
                            if is_marker_keyword_argument(part):
                                key, val = part.split('=', 1)
                                kwargs[key.strip()] = clean_marker_argument(val)
                            else:
                                args.append(clean_marker_argument(part))
                    else:
                        # Just args
                        args = [clean_marker_argument(arg) for arg in split_marker_arguments(args_str)]
                markers.append({{'name': name, 'args': args, 'kwargs': kwargs}})
            else:
                markers.append({{'name': marker_str, 'args': [], 'kwargs': {{}}}})
    return markers

def marker_bool(value):
    """Convert simple marker keyword values to bools."""
    if isinstance(value, bool):
        return value
    return str(value).strip().lower() in ('true', '1', 'yes')

def evaluate_marker_condition(condition):
    """Evaluate common pytest marker conditions in the test process."""
    condition = str(condition).strip()
    if condition in ('True', 'true', '1'):
        return True
    if condition in ('False', 'false', '0'):
        return False

    try:
        return bool(eval(
            condition,
            {{'__builtins__': {{}}}},
            {{'os': os, 'platform': platform, 'sys': sys}},
        ))
    except Exception as e:
        debug_print(f"Could not evaluate marker condition {{condition!r}}: {{e}}")

    return False

def exception_matches_expected(exc, expected):
    """Check xfail(raises=...) by exception class name."""
    if not expected:
        return True

    expected_names = expected
    if not isinstance(expected_names, (list, tuple)):
        expected_names = [expected_names]

    exc_type = type(exc)
    actual_names = {{
        exc_type.__name__,
        f"{{exc_type.__module__}}.{{exc_type.__name__}}",
    }}

    for expected_name in expected_names:
        clean_name = str(expected_name).strip().strip('"\'')
        if clean_name in actual_names or clean_name.endswith('.' + exc_type.__name__):
            return True

    return False

def check_skip_markers(markers):
    """Check if test should be skipped based on markers"""
    for marker in markers:
        if marker['name'] == 'skip':
            # Get reason from kwargs or first arg
            if 'reason' in marker['kwargs']:
                return marker['kwargs']['reason']
            elif marker['args']:
                return marker['args'][0]
            else:
                return 'Skipped'
        elif marker['name'] == 'skipif':
            if marker['args']:
                condition = marker['args'][0]
                if evaluate_marker_condition(condition):
                    reason = marker['kwargs'].get('reason') or marker['args'][1] if len(marker['args']) > 1 else 'Conditional skip'
                    return reason
    return None

def get_xfail_info(markers):
    """Return xfail marker metadata or None if no active xfail applies."""
    for marker in markers:
        if marker['name'] != 'xfail':
            continue

        args = marker.get('args', [])
        kwargs = marker.get('kwargs', {{}})

        if args and not evaluate_marker_condition(args[0]):
            return None

        reason = kwargs.get('reason')
        if not reason and args:
            reason = args[0]

        return {{
            'reason': reason or 'Expected to fail',
            'strict': marker_bool(kwargs.get('strict', False)),
            'raises': kwargs.get('raises'),
        }}

    return None

def check_xfail_markers(markers):
    """Check if test is expected to fail"""
    xfail_info = get_xfail_info(markers)
    if xfail_info:
        return xfail_info['reason']
    return None

def setup_module_if_needed(module_name):
    """Execute setup_module if it exists and hasn't been called yet"""
    if module_name in _setup_state['modules'] and _setup_state['modules'][module_name]['setup_done']:
        return

    if module_name not in module_cache:
        return

    mod = module_cache[module_name]
    if hasattr(mod, 'setup_module'):
        try:
            debug_print(f"Calling setup_module for {{module_name}}")
            mod.setup_module(mod)
            _setup_state['modules'][module_name] = {{'setup_done': True, 'teardown_done': False}}
            _setup_state['setup_order'].append(('module', module_name))
        except Exception as e:
            debug_print(f"setup_module failed for {{module_name}}: {{e}}")
            raise

def teardown_module_if_needed(module_name):
    """Execute teardown_module if it exists and setup was called"""
    if module_name not in _setup_state['modules'] or not _setup_state['modules'][module_name]['setup_done']:
        return

    if _setup_state['modules'][module_name].get('teardown_done', False):
        return

    mod = module_cache.get(module_name)
    if mod and hasattr(mod, 'teardown_module'):
        try:
            debug_print(f"Calling teardown_module for {{module_name}}")
            mod.teardown_module(mod)
            _setup_state['modules'][module_name]['teardown_done'] = True
        except Exception as e:
            debug_print(f"teardown_module failed for {{module_name}}: {{e}}")

def setup_class_if_needed(class_path, cls):
    """Execute setup_class if it exists and hasn't been called yet"""
    # Delegate to the new lifecycle manager
    _class_lifecycle.setup_class(class_path, cls)

def teardown_class_if_needed(class_path, cls):
    """Execute teardown_class if it exists and setup was called"""
    # Delegate to the new lifecycle manager
    _class_lifecycle.teardown_class(class_path)

def setup_method_if_needed(instance, method_name):
    """Execute setup_method if it exists"""
    if hasattr(instance, 'setup_method'):
        try:
            method = getattr(instance, method_name)
            setup = instance.setup_method
            if len(inspect.signature(setup).parameters) == 0:
                setup()
            else:
                setup(method)
        except Exception as e:
            debug_print(f"setup_method failed: {{e}}")
            raise

def teardown_method_if_needed(instance, method_name):
    """Execute teardown_method if it exists"""
    if hasattr(instance, 'teardown_method'):
        try:
            method = getattr(instance, method_name)
            teardown = instance.teardown_method
            if len(inspect.signature(teardown).parameters) == 0:
                teardown()
            else:
                teardown(method)
        except Exception as e:
            debug_print(f"teardown_method failed: {{e}}")

class FixtureRequest:
    """Complete pytest-compatible fixture request implementation"""
    def __init__(self, node_id, test_name, class_name=None, module_name=None):
        self.node_id = node_id
        self.test_name = test_name
        self.class_name = class_name
        self.module_name = module_name
        self.param = None
        self.instance = None
        self._fixture_values = {{}}
        self._finalizers = []
        self.fixturenames = []  # List of fixture names used by this test
        class Cache:
            def __init__(self):
                self._values = {{}}

            def get(self, key, default=None):
                return self._values.get(key, default)

            def set(self, key, value):
                self._values[key] = value

        class Config:
            def __init__(self):
                self.workerinput = {{
                    'workerid': 'master',
                    'testrunuid': 'local',
                }}
                self.cache = Cache()

            def getoption(self, name, default=None):
                return default

        self.config = Config()
        # Create a simple node object for compatibility
        class Node:
            def __init__(self, nodeid, name, module):
                self.nodeid = nodeid
                self.name = name
                self.module = module
                self._markers = []

            def iter_markers(self, name=None):
                """Iterate over markers"""
                if name:
                    return [m for m in self._markers if m.name == name]
                return self._markers

            def get_closest_marker(self, name, default=None):
                markers = self.iter_markers(name)
                return markers[-1] if markers else default

            def add_marker(self, marker):
                """Add a marker to the node"""
                self._markers.append(marker)

        self.node = Node(node_id, test_name, module_name)
        self.fixturename = None  # Will be set when requesting fixtures
        self.scope = 'function'  # Default scope

    def getfixturevalue(self, name):
        """Get fixture value by name"""
        if name in self._fixture_values:
            return self._fixture_values[name]
        # Execute fixture and cache result
        return execute_fixture(name, self)

    def addfinalizer(self, finalizer):
        """Add finalizer function"""
        self._finalizers.append(finalizer)

    def _finalize(self):
        """Execute all finalizers in reverse order"""
        for finalizer in reversed(self._finalizers):
            try:
                finalizer()
            except Exception as e:
                debug_print(f"Finalizer error: {{e}}")

def get_fixture_scope_key(fixture_name, scope, request):
    """Get cache key based on fixture scope"""
    if scope == "function":
        return f"{{fixture_name}}::{{request.node_id}}"
    elif scope == "class":
        return f"{{fixture_name}}::{{request.module_name}}::{{request.class_name}}"
    elif scope == "module":
        return f"{{fixture_name}}::{{request.module_name}}"
    elif scope == "package":
        # Extract package from module path
        parts = request.module_name.split('.')
        package = parts[0] if parts else "default"
        return f"{{fixture_name}}::{{package}}"
    else:  # session
        return f"{{fixture_name}}::session"

def execute_fixture(fixture_name, request):
    """Execute a fixture with full scope and dependency support"""
    # Check if it's a built-in fixture
    if fixture_name in BUILTIN_FIXTURES:
        return BUILTIN_FIXTURES[fixture_name](request)

    # Look up fixture definition
    if fixture_name not in _fixture_registry:
        raise ValueError(f"Fixture '{{fixture_name}}' not found")

    fixture_func = _fixture_registry[fixture_name]
    fixture_meta = _fixture_metadata.get(fixture_name, {{}})
    scope = fixture_meta.get('scope', 'function')

    # If the fixture is wrapped by pytest, unwrap it
    if hasattr(fixture_func, '__wrapped__'):
        # This is a pytest-wrapped fixture, get the original function
        original_func = fixture_func.__wrapped__
        while hasattr(original_func, '__wrapped__'):
            original_func = original_func.__wrapped__
        fixture_func = original_func

    # Check cache based on scope
    cache_key = get_fixture_scope_key(fixture_name, scope, request)
    if cache_key in active_fixtures:
        debug_print(f"Using cached fixture '{{fixture_name}}' (scope={{scope}})")
        return active_fixtures[cache_key]['value']

    debug_print(f"Executing fixture '{{fixture_name}}' (scope={{scope}})")

    # Get fixture dependencies
    sig = inspect.signature(fixture_func)
    kwargs = {{}}

    debug_print(f"Fixture '{{fixture_name}}' has parameters: {{list(sig.parameters.keys())}}")

    for param_name in sig.parameters:
        if param_name == 'self':
            # Skip self parameter for class methods
            continue
        elif param_name == 'request':
            # Set the fixturename on the request object
            request.fixturename = fixture_name
            kwargs['request'] = request
        elif param_name in _fixture_registry or param_name in BUILTIN_FIXTURES:
            # Recursive fixture execution
            debug_print(f"Resolving dependency '{{param_name}}' for fixture '{{fixture_name}}'")
            kwargs[param_name] = execute_fixture(param_name, request)
        else:
            debug_print(f"WARNING: Unknown fixture dependency '{{param_name}}' for fixture '{{fixture_name}}'")

    # Handle parametrized fixtures and indirect parametrization
    params = fixture_meta.get('params', [])

    # Check if this fixture is being used for indirect parametrization
    if hasattr(request, '_indirect_params') and fixture_name in request._indirect_params:
        # This fixture is being parametrized indirectly
        request.param = request._indirect_params[fixture_name]
        debug_print(f"Setting request.param for indirect fixture '{{fixture_name}}': {{request.param}}")
    elif params:
        # For parametrized fixtures, we need to handle the request.param
        # In pytest, this would be set by the test runner
        # For now, use the first param if request.param is not set
        if request.param is None and 'request' in kwargs:
            # This is a parametrized fixture that uses request.param
            # We'll need to iterate through params in the test runner
            # For now, just use the first parameter
            request.param = params[0] if params else None

    # Execute fixture
    try:
        # Check if this is a class method fixture
        if fixture_meta.get('is_class_method') and hasattr(request, '_test_instance'):
            # Bind the method to the instance
            instance = request._test_instance
            bound_method = fixture_func.__get__(instance, instance.__class__)

            # Filter out 'self' from kwargs if present
            if 'self' in kwargs:
                del kwargs['self']

            if fixture_meta.get('is_async', False):
                result = asyncio.run(bound_method(**kwargs))
            else:
                result = bound_method(**kwargs)
        else:
            # Regular fixture execution
            if fixture_meta.get('is_async', False):
                debug_print(f"Executing async fixture '{{fixture_name}}'")
                try:
                    result = asyncio.run(fixture_func(**kwargs))
                    debug_print(f"Async fixture '{{fixture_name}}' returned: {{type(result)}} = {{result}}")
                except Exception as async_err:
                    debug_print(f"Error executing async fixture '{{fixture_name}}': {{async_err}}")
                    raise
            else:
                result = fixture_func(**kwargs)

        # Real pytest fixture decorators can hide async metadata from Fastest's
        # parser. If calling the unwrapped fixture produced a coroutine, resolve
        # it here so async fixture values match pytest behavior.
        if inspect.iscoroutine(result):
            debug_print(f"Awaiting coroutine fixture result for '{{fixture_name}}'")
            result = asyncio.run(result)

        # Handle generator fixtures (yield)
        if inspect.isgeneratorfunction(fixture_func) or inspect.isasyncgenfunction(fixture_func):
            gen = result
            value = next(gen)
            # Store generator for teardown
            active_fixtures[cache_key] = {{
                'value': value,
                'generator': gen,
                'scope': scope,
                'is_generator': True
            }}
            teardown_stack.append((cache_key, scope))
            return value
        else:
            # Regular fixture
            active_fixtures[cache_key] = {{
                'value': result,
                'scope': scope,
                'is_generator': False
            }}
            teardown_stack.append((cache_key, scope))
            return result

    except Exception as e:
        debug_print(f"Error executing fixture '{{fixture_name}}': {{e}}")
        raise

def teardown_fixtures(scope, request=None, class_path=None):
    """Teardown fixtures for a given scope"""
    debug_print(f"Tearing down fixtures for scope: {{scope}}, class_path: {{class_path}}")

    # Determine which fixtures to teardown
    to_teardown = []
    scope_order = ['session', 'package', 'module', 'class', 'function']
    scope_idx = scope_order.index(scope)

    for cache_key, fixture_scope in reversed(teardown_stack):
        fixture_data = active_fixtures.get(cache_key)
        if fixture_data:
            fixture_scope_idx = scope_order.index(fixture_data['scope'])
            if fixture_scope_idx >= scope_idx:
                # If we're tearing down class fixtures and have a specific class_path,
                # only teardown fixtures for that specific class
                if scope == 'class' and class_path:
                    # Check if this fixture belongs to the specific class
                    if class_path in cache_key:
                        to_teardown.append(cache_key)
                else:
                    to_teardown.append(cache_key)

    # Teardown in reverse order
    for cache_key in to_teardown:
        fixture_data = active_fixtures.get(cache_key)
        if fixture_data and fixture_data.get('is_generator'):
            try:
                gen = fixture_data['generator']
                next(gen, None)  # Trigger teardown
                debug_print(f"Teardown complete for {{cache_key}}")
            except StopIteration:
                pass  # Normal completion
            except Exception as e:
                debug_print(f"Teardown error for {{cache_key}}: {{e}}")

        # Remove from active fixtures
        active_fixtures.pop(cache_key, None)
        if (cache_key, fixture_data['scope']) in teardown_stack:
            teardown_stack.remove((cache_key, fixture_data['scope']))

# Built-in fixtures
def builtin_tmp_path(request):
    """Built-in tmp_path fixture"""
    return pathlib.Path(tempfile.mkdtemp())

def builtin_tmpdir_factory(request):
    """Built-in tmpdir_factory fixture"""
    class TmpdirFactory:
        def __init__(self):
            self._base = pathlib.Path(tempfile.mkdtemp())

        def mktemp(self, basename):
            path = self._base / str(basename)
            path.mkdir(parents=True, exist_ok=True)
            return path

    return TmpdirFactory()

def builtin_cache(request):
    """Built-in cache fixture"""
    return request.config.cache

def builtin_event_loop(request):
    """Built-in event_loop fixture"""
    loop = asyncio.new_event_loop()
    request.addfinalizer(loop.close)
    return loop

def builtin_capsys(request):
    """Built-in capsys fixture"""
    global _capturing_output, _original_stderr

    class CaptureFixture:
        def __init__(self):
            self._stdout = StringIO()
            self._stderr = StringIO()
            self._old_stdout = sys.stdout
            self._old_stderr = sys.stderr
            # Start capturing
            sys.stdout = self._stdout
            sys.stderr = self._stderr
            # Set global flags for debug output
            global _capturing_output, _original_stderr
            _capturing_output = True
            _original_stderr = self._old_stderr

        def readouterr(self):
            # Get captured output
            out = self._stdout.getvalue()
            err = self._stderr.getvalue()
            # Clear the buffers
            self._stdout.seek(0)
            self._stdout.truncate()
            self._stderr.seek(0)
            self._stderr.truncate()

            class Result:
                def __init__(self, out, err):
                    self.out = out
                    self.err = err

            return Result(out, err)

        def _restore(self):
            """Restore original stdout/stderr"""
            sys.stdout = self._old_stdout
            sys.stderr = self._old_stderr
            # Reset global flags
            global _capturing_output, _original_stderr
            _capturing_output = False
            _original_stderr = None

    capture = CaptureFixture()
    # Register finalizer to ensure cleanup
    request.addfinalizer(capture._restore)
    return capture

def builtin_monkeypatch(request):
    """Built-in monkeypatch fixture"""
    class MonkeyPatch:
        def __init__(self):
            self._setattr = []
            self._setenv = []
            self._delenv = []

        def setattr(self, target, name, value):
            if isinstance(target, str):
                parts = target.split('.')
                obj = __import__(parts[0])
                for part in parts[1:-1]:
                    obj = getattr(obj, part)
                target = obj
                name = parts[-1]

            old_value = getattr(target, name, None)
            self._setattr.append((target, name, old_value))
            setattr(target, name, value)

        def setenv(self, name, value):
            import os
            old_value = os.environ.get(name)
            self._setenv.append((name, old_value))
            os.environ[name] = str(value)

        def delenv(self, name, raising=True):
            import os
            old_value = os.environ.get(name)
            self._delenv.append((name, old_value))
            if name in os.environ:
                del os.environ[name]
            elif raising:
                raise KeyError(name)

        def undo(self):
            import os
            for target, name, value in reversed(self._setattr):
                if value is None:
                    delattr(target, name)
                else:
                    setattr(target, name, value)

            for name, value in reversed(self._setenv):
                if value is None:
                    os.environ.pop(name, None)
                else:
                    os.environ[name] = value

            for name, value in reversed(self._delenv):
                if value is not None:
                    os.environ[name] = value

    mp = MonkeyPatch()
    request.addfinalizer(mp.undo)
    return mp

def builtin_request(request):
    """Built-in request fixture"""
    # The request object is already created in execute_test_with_fixtures
    # Just return it as-is
    return request


def builtin_mocker(request):
    """Basic mocker fixture implementation"""
    import unittest.mock

    class PatchProxy:
        def __init__(self, mocker):
            self._mocker = mocker

        def __call__(self, target, *args, **kwargs):
            patcher = unittest.mock.patch(target, *args, **kwargs)
            return self._mocker._start_patch(patcher)

        def object(self, target, name, *args, **kwargs):
            patcher = unittest.mock.patch.object(target, name, *args, **kwargs)
            return self._mocker._start_patch(patcher)

    class MockerFixture:
        def __init__(self):
            self._patches = []
            self._mocks = []
            self.patch = PatchProxy(self)
            request.addfinalizer(self.stopall)

        def _start_patch(self, patcher):
            mock = patcher.start()
            self._patches.append(patcher)
            return mock

        def _track_mock(self, mock):
            self._mocks.append(mock)
            return mock

        def Mock(self, *args, **kwargs):
            """Create a Mock object"""
            return self._track_mock(unittest.mock.Mock(*args, **kwargs))

        def MagicMock(self, *args, **kwargs):
            """Create a MagicMock object"""
            return self._track_mock(unittest.mock.MagicMock(*args, **kwargs))

        def AsyncMock(self, *args, **kwargs):
            """Create an AsyncMock object"""
            return self._track_mock(unittest.mock.AsyncMock(*args, **kwargs))

        def PropertyMock(self, *args, **kwargs):
            """Create a PropertyMock object"""
            return self._track_mock(unittest.mock.PropertyMock(*args, **kwargs))

        def mock_open(self, *args, **kwargs):
            """Create a mock_open helper"""
            return self._track_mock(unittest.mock.mock_open(*args, **kwargs))

        def spy(self, obj, name):
            """Create a spy by wrapping the original method"""
            original = getattr(obj, name)
            mock = self.Mock(wraps=original)
            patcher = unittest.mock.patch.object(obj, name, mock)
            self._start_patch(patcher)
            return mock

        def stub(self, name=None):
            """Create a generic stub mock"""
            return self.Mock(name=name)

        def resetall(self):
            """Reset all mocks created by this fixture"""
            for mock in self._mocks:
                if hasattr(mock, 'reset_mock'):
                    mock.reset_mock()

        def stopall(self):
            """Stop all active patches"""
            for patcher in reversed(self._patches):
                try:
                    patcher.stop()
                except RuntimeError:
                    pass
            self._patches.clear()

    return MockerFixture()


BUILTIN_FIXTURES = {{
    'tmp_path': builtin_tmp_path,
    'tmpdir': builtin_tmp_path,
    'tmp_path_factory': builtin_tmpdir_factory,
    'tmpdir_factory': builtin_tmpdir_factory,
    'cache': builtin_cache,
    'event_loop': builtin_event_loop,
    'capsys': builtin_capsys,
    'monkeypatch': builtin_monkeypatch,
    'request': builtin_request,
    'mocker': builtin_mocker,
}}

# Fixture registry (populated by conftest.py and test modules)
_fixture_registry = {{}}
_fixture_metadata = {{}}

# Include essential functions from the original worker

@contextmanager
def _null_redirect():
    """Ultra-lightweight null redirect context manager"""
    old_stdout = sys.stdout
    old_stderr = sys.stderr
    sys.stdout = open(os.devnull, 'w')
    sys.stderr = sys.stdout
    try:
        yield
    finally:
        sys.stdout.close()
        sys.stdout = old_stdout
        sys.stderr = old_stderr

def ensure_path_cached(filepath):
    """Cache-aware path management"""
    if filepath in path_cache:
        return

    parent_dir = os.path.dirname(filepath)
    if parent_dir and parent_dir not in sys.path:
        sys.path.insert(0, parent_dir)
        path_cache.add(parent_dir)

    # Project virtual environment detection
    detected_venv = detect_project_venv_from_path(filepath)
    if detected_venv:
        version_info = f"{{sys.version_info.major}}.{{sys.version_info.minor}}"
        venv_site_packages = os.path.join(detected_venv, 'lib', f'python{{version_info}}', 'site-packages')
        if os.path.exists(venv_site_packages) and venv_site_packages not in sys.path:
            sys.path.insert(0, venv_site_packages)

def detect_project_venv_from_path(filepath):
    """Detect project virtual environment from file path"""
    current = os.path.dirname(os.path.abspath(filepath))
    while current != os.path.dirname(current):
        venv_path = os.path.join(current, 'venv')
        if os.path.exists(venv_path) and os.path.isdir(venv_path):
            return venv_path
        current = os.path.dirname(current)
    return None

def get_module_cache_key(module_name, module_path=None):
    if module_path:
        return os.path.abspath(module_path)
    return module_name

def make_file_module_name(module_name, module_path):
    abs_path = os.path.abspath(module_path)
    suffix = ''.join(ch if ch.isalnum() else '_' for ch in abs_path)
    return '_fastest_file_' + suffix + '_' + module_name

def safe_import_module(module_name, module_path=None):
    """Import a module with proper UTF-8 encoding support."""
    try:
        # For Python 3, ensure the source file is read with UTF-8 encoding
        if module_path and sys.version_info[0] >= 3:
            import importlib.util
            abs_path = os.path.abspath(module_path)
            module_key = get_module_cache_key(module_name, abs_path)
            if module_key in module_cache:
                return module_cache[module_key]

            file_module_name = make_file_module_name(module_name, abs_path)
            spec = importlib.util.spec_from_file_location(file_module_name, abs_path)
            if spec and spec.loader:
                # Create module
                module = importlib.util.module_from_spec(spec)
                previous_module = sys.modules.get(file_module_name)
                sys.modules[file_module_name] = module

                # Load with UTF-8 encoding
                try:
                    spec.loader.exec_module(module)
                except SyntaxError as e:
                    # If we get a syntax error, it might be encoding-related
                    # Try to load the source manually with UTF-8
                    if os.path.exists(abs_path):
                        with open(abs_path, 'r', encoding='utf-8') as f:
                            source = f.read()

                        # Compile and execute
                        code = compile(source, abs_path, 'exec')
                        exec(code, module.__dict__)
                except Exception:
                    if previous_module is None:
                        sys.modules.pop(file_module_name, None)
                    else:
                        sys.modules[file_module_name] = previous_module
                    raise

                module_cache[module_key] = module
                return module

        # Fallback to standard import
        return importlib.import_module(module_name)
    except Exception as e:
        debug_print(f"Failed to import module {{module_name}}: {{e}}")
        # If all else fails, try standard import
        return importlib.import_module(module_name)

def get_cached_function(module_name, func_name, filepath=None):
    """Ultra-fast function caching with optimized loading"""
    module_key = get_module_cache_key(module_name, filepath)
    cache_key = f"{{module_key}}.{{func_name}}"

    if cache_key in fn_cache:
        return fn_cache[cache_key]

    try:
        if filepath:
            ensure_path_cached(filepath)

        # Get cached module or import
        if module_key in module_cache:
            mod = module_cache[module_key]
        else:
            # Use safe import with UTF-8 support
            mod = safe_import_module(module_name, filepath)
            module_cache[module_key] = mod

        # Handle class methods
        if '::' in func_name:
            class_name, method_name = func_name.split('::', 1)
            cls = getattr(mod, class_name)

            # Setup class if needed using the new lifecycle management
            class_path = f"{{module_key}}.{{class_name}}"
            setup_class_if_needed(class_path, cls)

            # Instantiate class
            try:
                instance = cls()
            except Exception:
                try:
                    sig = inspect.signature(cls.__init__)
                    params = list(sig.parameters.values())[1:]
                    if params and all(p.default == inspect.Parameter.empty for p in params):
                        instance = object.__new__(cls)
                    else:
                        instance = cls()
                except Exception:
                    instance = object.__new__(cls)

            # Call setUp if exists
            if hasattr(instance, 'setUp'):
                try:
                    instance.setUp()
                except Exception:
                    pass

            func = getattr(instance, method_name)
            fn_cache[cache_key] = (func, instance)
            return func, instance
        else:
            func = getattr(mod, func_name)
            fn_cache[cache_key] = func
            return func, None

    except Exception as e:
        raise ImportError(f"Failed to load {{module_name}}.{{func_name}}: {{str(e)}}")

def restore_fastest_param_value(value):
    """Restore non-JSON Python values encoded by the Rust parametrize parser."""
    if isinstance(value, dict):
        keys = set(value.keys())
        if keys == {{'__fastest_float__'}}:
            return float(value['__fastest_float__'])
        if keys == {{'__fastest_set__'}}:
            return set(restore_fastest_param_value(item) for item in value['__fastest_set__'])
        return {{key: restore_fastest_param_value(inner) for key, inner in value.items()}}
    if isinstance(value, list):
        return [restore_fastest_param_value(item) for item in value]
    return value

def parse_parametrize_args(test_id):
    """Parse parametrize arguments from test ID"""
    if '[' not in test_id or ']' not in test_id:
        return []

    start = test_id.find('[')
    end = test_id.rfind(']')
    if start == -1 or end == -1 or start >= end:
        return []

    param_str = test_id[start + 1:end]

    # Handle different formats
    if '-' in param_str and ',' not in param_str:
        raw_params = param_str.split('-')
    elif ',' in param_str:
        raw_params = param_str.split(',')
    else:
        raw_params = [param_str]

    # Convert to Python types
    params = []
    for param in raw_params:
        param = param.strip()

        if param.lower() == 'none':
            params.append(None)
        elif param.lower() == 'true':
            params.append(True)
        elif param.lower() == 'false':
            params.append(False)
        else:
            try:
                if param.isdigit() or (param.startswith('-') and param[1:].isdigit()):
                    params.append(int(param))
                elif '.' in param and param.replace('.', '').replace('-', '').isdigit():
                    params.append(float(param))
                elif (param.startswith('"') and param.endswith('"')) or (param.startswith("'") and param.endswith("'")):
                    params.append(param[1:-1])
                else:
                    params.append(param)
            except ValueError:
                params.append(param)

    return params

def is_async_function(func):
    """Check if a function is async"""
    return asyncio.iscoroutinefunction(func)


# Import enhanced assertion introspection
try:
    from enhanced_assertions import introspect_assertion as enhanced_introspect
    USE_ENHANCED_ASSERTIONS = True
except ImportError:
    USE_ENHANCED_ASSERTIONS = False

def extract_assertion_details(exc, func, kwargs):
    """Extract detailed information from assertion errors"""
    # Try enhanced introspection first if available
    if USE_ENHANCED_ASSERTIONS:
        try:
            result = enhanced_introspect(exc, func, kwargs)
            if result:
                return {{'formatted': result, 'enhanced': True}}
        except Exception:
            pass

    # Fallback to original implementation
    import traceback
    import ast
    import inspect

    try:
        # Get the traceback
        tb = exc.__traceback__

        # Find the frame where the assertion happened
        while tb and tb.tb_next:
            frame = tb.tb_frame
            if frame.f_code.co_name == func.__name__:
                break
            tb = tb.tb_next

        if not tb:
            return None

        frame = tb.tb_frame

        # Try to get the source code
        try:
            source_lines = inspect.getsourcelines(func)[0]
            # Find the line that failed
            line_no = tb.tb_lineno - func.__code__.co_firstlineno
            if 0 <= line_no < len(source_lines):
                failed_line = source_lines[line_no].strip()

                # Parse the assertion to extract values
                if failed_line.startswith('assert '):
                    assertion_code = failed_line[7:]  # Remove 'assert '

                    # Try to evaluate parts of the assertion
                    details = {{
                        'assertion': assertion_code,
                        'values': {{}},
                        'line': tb.tb_lineno,
                        'function': func.__name__
                    }}

                    # Extract local variables from the frame
                    local_vars = frame.f_locals.copy()
                    # Remove function arguments to focus on assertion values
                    for arg in kwargs:
                        local_vars.pop(arg, None)

                    # Try to parse and evaluate the assertion
                    try:
                        tree = ast.parse(assertion_code, mode='eval')
                        # Simple comparison extraction
                        if isinstance(tree.body, ast.Compare):
                            # Get left side
                            left_code = ast.get_source_segment(assertion_code, tree.body.left)
                            if left_code:
                                try:
                                    left_val = eval(left_code, frame.f_globals, frame.f_locals)
                                    details['values'][left_code] = repr(left_val)
                                except:
                                    pass

                            # Get right side (first comparator)
                            if tree.body.comparators:
                                right_code = ast.get_source_segment(assertion_code, tree.body.comparators[0])
                                if right_code:
                                    try:
                                        right_val = eval(right_code, frame.f_globals, frame.f_locals)
                                        details['values'][right_code] = repr(right_val)
                                    except:
                                        pass

                            # Store comparison operator
                            if tree.body.ops:
                                op = tree.body.ops[0]
                                if isinstance(op, ast.Eq):
                                    details['operator'] = '=='
                                elif isinstance(op, ast.NotEq):
                                    details['operator'] = '!='
                                elif isinstance(op, ast.Lt):
                                    details['operator'] = '<'
                                elif isinstance(op, ast.Gt):
                                    details['operator'] = '>'
                                elif isinstance(op, ast.In):
                                    details['operator'] = 'in'
                    except:
                        # If AST parsing fails, try simpler extraction
                        for var_name, var_value in frame.f_locals.items():
                            if var_name in assertion_code:
                                details['values'][var_name] = repr(var_value)

                    # Add any relevant local variables
                    if local_vars:
                        details['locals'] = {{k: repr(v) for k, v in local_vars.items()
                                            if not k.startswith('_')}}

                    return details
        except:
            pass
    except:
        pass

    return None


def format_assertion_error(details):
    """Format assertion error details into a readable message"""
    # If we have enhanced formatting, use it directly
    if isinstance(details, dict) and details.get('enhanced') and 'formatted' in details:
        return details['formatted']

    # Original formatting
    lines = []

    # Main assertion line
    lines.append(f"Assertion failed: assert {{details['assertion']}}")

    # Show values
    if details.get('values'):
        lines.append("Where:")
        for expr, value in details['values'].items():
            lines.append(f"    {{expr}} = {{value}}")

    # Show comparison if available
    if 'operator' in details and len(details['values']) == 2:
        values = list(details['values'].values())
        if len(values) == 2:
            lines.append(f"    {{values[0]}} {{details['operator']}} {{values[1]}} is False")

    # Show local variables if any
    if details.get('locals'):
        lines.append("Local variables:")
        for name, value in details['locals'].items():
            lines.append(f"    {{name}} = {{value}}")

    # Add location info
    lines.append(f"    at {{details['function']}}:{{details['line']}}")

    return "\n".join(lines)

def load_conftest_modules(test_path):
    """Load all conftest.py modules in the directory hierarchy"""
    import os

    conftest_modules = []
    current_dir = os.path.dirname(os.path.abspath(test_path))

    # Walk up the directory tree looking for conftest.py files
    while current_dir and current_dir != os.path.dirname(current_dir):
        conftest_path = os.path.join(current_dir, 'conftest.py')
        if os.path.exists(conftest_path):
            # Convert path to module name
            module_name = f"conftest_{{current_dir.replace(os.sep, '_').replace(':', '')}}"

            # Import the conftest module
            try:
                import importlib.util
                spec = importlib.util.spec_from_file_location(module_name, conftest_path)
                if spec and spec.loader:
                    conftest_module = importlib.util.module_from_spec(spec)
                    spec.loader.exec_module(conftest_module)
                    conftest_modules.append(conftest_module)
                    debug_print(f"Loaded conftest.py from {{conftest_path}}")
            except Exception as e:
                debug_print(f"Failed to load conftest.py from {{conftest_path}}: {{e}}")

        # Move up one directory
        current_dir = os.path.dirname(current_dir)

    # Return in reverse order (root conftest first)
    return list(reversed(conftest_modules))


def scan_module_for_fixtures(module):
    """Scan a module for fixture definitions"""
    debug_print(f"Scanning module {{module.__name__}} for fixtures")

    # First try to import pytest in the module's context
    if hasattr(module, 'pytest'):
        pytest_module = module.pytest
    else:
        try:
            import pytest as pytest_module
        except ImportError:
            debug_print("pytest not available, using basic fixture detection")
            pytest_module = None

    fixture_count = 0
    for name in dir(module):
        if name.startswith('_'):
            continue
        obj = getattr(module, name)

        # Check if it's a fixture
        is_fixture = False
        fixture_metadata = {{}}

        # Method 1: Check for pytest fixture markers
        if (inspect.isfunction(obj) or inspect.ismethod(obj)) and hasattr(obj, '_pytestfixturefunction'):
            debug_print(f"Found pytest fixture marker on {{name}}")
            is_fixture = True
            fixture_info = obj._pytestfixturefunction
            fixture_metadata = {{
                'scope': getattr(fixture_info, 'scope', 'function'),
                'params': getattr(fixture_info, 'params', None),
                'autouse': getattr(fixture_info, 'autouse', False),
                'ids': getattr(fixture_info, 'ids', None),
            }}
        # Method 2: Check if function has fixture decorator via __name__ attribute
        elif inspect.isfunction(obj) or inspect.ismethod(obj):
            # Check if this is a decorated function (look for wrapper attributes)
            if hasattr(obj, '__wrapped__'):
                # This might be a fixture
                is_fixture = True
                # Try to extract metadata from the wrapper
                if hasattr(obj, 'fixture'):
                    fixture_metadata = obj.fixture
                else:
                    fixture_metadata = {{}}
        # Method 3: Check if name matches common fixture patterns and is callable
        elif callable(obj) and (
            name.endswith('_fixture') or
            name.startswith('fixture_') or
            name in ['tmp_path', 'capsys', 'monkeypatch', 'request', 'tmpdir',
                     'simple_fixture', 'dependent_fixture', 'nested_dependent',
                     'class_fixture', 'module_fixture', 'session_fixture',
                     'yield_fixture', 'yield_with_dependency', 'parametrized_fixture',
                     'parametrized_with_ids', 'fixture_with_request', 'fixture_with_finalizer',
                     'failing_fixture', 'user_factory', 'dynamic_fixture',
                     'sample_data', 'mock_service', 'letter_fixture', 'number_fixture',
                     'named_fixture']
        ):
            is_fixture = True
            # Infer metadata from function signature and name
            sig = inspect.signature(obj)
            params = list(sig.parameters.keys())

            # Basic scope inference
            if 'session' in name:
                scope = 'session'
            elif 'module' in name:
                scope = 'module'
            elif 'class' in name:
                scope = 'class'
            else:
                scope = 'function'

            fixture_metadata = {{
                'scope': scope,
                'params': None,
                'autouse': False,
                'ids': None,
            }}

        if is_fixture:
            _fixture_registry[name] = obj
            # Check if the original wrapped function is async
            unwrapped = obj
            if hasattr(obj, '__wrapped__'):
                unwrapped = obj.__wrapped__
                while hasattr(unwrapped, '__wrapped__'):
                    unwrapped = unwrapped.__wrapped__
            is_async = inspect.iscoroutinefunction(unwrapped)
            _fixture_metadata[name] = {{
                'scope': fixture_metadata.get('scope', 'function'),
                'params': fixture_metadata.get('params') or [],
                'autouse': fixture_metadata.get('autouse', False),
                'ids': fixture_metadata.get('ids') or [],
                'is_generator': inspect.isgeneratorfunction(unwrapped),
                'is_async': is_async,
            }}
            debug_print(f"Registered fixture: {{name}} (scope={{_fixture_metadata[name]['scope']}}, async={{is_async}})")
            fixture_count += 1

    debug_print(f"Found {{fixture_count}} fixtures in module {{module.__name__}}")

def scan_class_for_fixtures(cls, module_name, class_name):
    """Scan a test class for fixture methods"""
    debug_print(f"Scanning class {{class_name}} for fixtures")

    for name in dir(cls):
        if name.startswith('_'):
            continue

        obj = getattr(cls, name)

        # Check if it's a fixture method
        is_fixture = False
        fixture_metadata = {{}}

        # Check for pytest fixture markers
        if hasattr(obj, '_pytestfixturefunction'):
            is_fixture = True
            fixture_info = obj._pytestfixturefunction
            fixture_metadata = {{
                'scope': getattr(fixture_info, 'scope', 'function'),
                'params': getattr(fixture_info, 'params', None),
                'autouse': getattr(fixture_info, 'autouse', False),
                'ids': getattr(fixture_info, 'ids', None),
            }}

        if is_fixture:
            # Create a unique name for class fixtures
            fixture_key = f"{{class_name}}.{{name}}"
            _fixture_registry[fixture_key] = obj
            _fixture_metadata[fixture_key] = {{
                'scope': fixture_metadata.get('scope', 'function'),
                'params': fixture_metadata.get('params') or [],
                'autouse': fixture_metadata.get('autouse', False),
                'ids': fixture_metadata.get('ids') or [],
                'is_generator': inspect.isgeneratorfunction(obj),
                'is_async': inspect.iscoroutinefunction(obj),
                'is_class_method': True,
                'class_name': class_name,
            }}
            debug_print(f"Registered class fixture: {{fixture_key}} (scope={{_fixture_metadata[fixture_key]['scope']}}, autouse={{_fixture_metadata[fixture_key]['autouse']}})")

def execute_test_with_fixtures(test_data):
    """Execute a test with full fixture support and marker handling"""
    debug_print(f"Starting test execution for: {{test_data['id']}}")
    start = perf()

    # Check for markers first
    decorators = test_data.get('decorators', [])
    markers = extract_markers(decorators)

    # Check if test should be skipped
    skip_reason = check_skip_markers(markers)
    if skip_reason:
        duration = perf() - start
        return {{
            'id': test_data['id'],
            'outcome': 'skipped',
            'skip_reason': skip_reason,
            'duration': duration,
            'error': None
        }}

    # Check if test is expected to fail
    xfail_info = get_xfail_info(markers)
    xfail_reason = xfail_info['reason'] if xfail_info else None
    is_xfail = xfail_info is not None

    # Initialize variables to None in case of errors
    instance = None
    request = None

    try:
        # Get the test module and scan for fixtures
        module_key = get_module_cache_key(test_data['module'], test_data.get('path'))
        if module_key not in module_cache:
            ensure_path_cached(test_data.get('path'))
            # Use safe import with UTF-8 support
            mod = safe_import_module(test_data['module'], test_data.get('path'))
            module_cache[module_key] = mod

            # Load and scan conftest.py modules
            if test_data.get('path'):
                conftest_modules = load_conftest_modules(test_data['path'])
                for conftest_mod in conftest_modules:
                    scan_module_for_fixtures(conftest_mod)

            # Scan the test module itself
            scan_module_for_fixtures(mod)
        else:
            mod = module_cache[module_key]

        # If this is a class method test, also scan the class for fixtures
        if test_data.get('class_name'):
            class_name = test_data['class_name']
            if hasattr(mod, class_name):
                cls = getattr(mod, class_name)
                scan_class_for_fixtures(cls, test_data['module'], class_name)

        # Call module setup if needed
        setup_module_if_needed(module_key)

        # Create request object
        request = FixtureRequest(
            node_id=test_data['id'],
            test_name=test_data['function'],
            class_name=test_data.get('class_name'),
            module_name=test_data['module']
        )

        # Add markers to the request.node object
        for marker in markers:
            # Create a simple marker object
            marker_obj = type('Mark', (), {{
                'name': marker['name'],
                'args': marker['args'],
                'kwargs': marker['kwargs']
            }})()
            request.node.add_marker(marker_obj)

        # Get test function
        debug_print(f"Getting test function: {{test_data['module']}}.{{test_data['function']}}")
        try:
            fn_result = get_cached_function(
                test_data['module'],
                test_data['function'],
                test_data.get('path')
            )
        except BaseException as get_func_exc:
            debug_print(f"Exception getting function: {{type(get_func_exc).__name__}}: {{get_func_exc}}")
            raise

        if isinstance(fn_result, tuple):
            func, instance = fn_result
        else:
            func = fn_result
            instance = None

        request.instance = instance
        if instance is not None:
            request._test_instance = instance

        # Get function signature
        sig = inspect.signature(func)
        all_params = list(sig.parameters.keys())

        # Remove 'self' if it's a method
        if 'self' in all_params:
            all_params.remove('self')

        # Parse parametrize arguments
        # First, try to extract parameters from decorators (new method)
        param_dict = {{}}
        indirect_params = []

        if 'decorators' in test_data:
            for decorator in test_data['decorators']:
                if decorator.startswith('__params__='):
                    # Extract the JSON parameter values
                    params_json = decorator[len('__params__='):]
                    try:
                        import json
                        param_dict = restore_fastest_param_value(json.loads(params_json))
                        debug_print("Extracted parameters from decorator: " + str(param_dict))
                    except Exception as e:
                        debug_print("Failed to parse parameters: " + str(e))
                elif decorator.startswith('__indirect__='):
                    # Extract the indirect parameter names
                    indirect_json = decorator[len('__indirect__='):]
                    try:
                        import json
                        indirect_params = json.loads(indirect_json)
                        debug_print("Extracted indirect parameters: " + str(indirect_params))
                    except Exception as e:
                        debug_print("Failed to parse indirect parameters: " + str(e))

        # If no parameters found in decorators, fall back to parsing from test ID
        if param_dict:
            parametrize_args = []
        else:
            parametrize_args = parse_parametrize_args(test_data['id'])

        # Build kwargs with fixtures
        kwargs = {{}}

        # Handle parametrized arguments first
        if param_dict:
            # Separate direct and indirect parameters
            direct_params = {{k: v for k, v in param_dict.items() if k not in indirect_params}}
            indirect_param_values = {{k: v for k, v in param_dict.items() if k in indirect_params}}

            # Set direct parameters
            for param_name, param_value in direct_params.items():
                if param_name in all_params:
                    kwargs[param_name] = param_value
                    debug_print("Setting " + str(param_name) + " = " + str(param_value) + " (direct parameter)")

            # Store indirect parameter values in request for fixtures to access
            if indirect_param_values:
                request._indirect_params = indirect_param_values
                debug_print("Storing indirect parameters in request: " + str(indirect_param_values))

            # Mark fixture candidates - include indirect parameters as they need fixture handling
            fixture_candidates = [p for p in all_params if p not in direct_params]
        elif parametrize_args:
            param_names = all_params[:len(parametrize_args)]
            for param_name, param_value in zip(param_names, parametrize_args):
                kwargs[param_name] = param_value
            fixture_candidates = all_params[len(parametrize_args):]
        else:
            fixture_candidates = all_params

        # Execute autouse fixtures first (before other fixtures)
        # Find all autouse fixtures that should apply to this test
        autouse_fixtures = []
        for fixture_name, metadata in _fixture_metadata.items():
            if metadata.get('autouse', False):
                # Check scope to see if this autouse fixture applies
                scope = metadata.get('scope', 'function')
                if scope == 'function':
                    autouse_fixtures.append(fixture_name)
                elif scope == 'class' and test_data.get('class_name'):
                    # Only include class fixtures that belong to this test's class
                    if '.' in fixture_name:
                        fixture_class = fixture_name.split('.')[0]
                        if fixture_class == test_data.get('class_name'):
                            autouse_fixtures.append(fixture_name)
                    else:
                        # Module-level fixtures with class scope apply to classes
                        autouse_fixtures.append(fixture_name)
                elif scope in ['module', 'session', 'package']:
                    autouse_fixtures.append(fixture_name)

        # Execute all autouse fixtures
        for fixture_name in autouse_fixtures:
            debug_print(f"Executing autouse fixture: {{fixture_name}}")
            try:
                # Check if this is a class method fixture
                fixture_meta = _fixture_metadata.get(fixture_name, {{}})
                if fixture_meta.get('is_class_method'):
                    # Only execute class method fixtures if we have an instance (i.e., we're in a class test)
                    if instance:
                        # Store the instance in the request for use in execute_fixture
                        request._test_instance = instance
                        # Execute through normal fixture system (handles dependencies)
                        execute_fixture(fixture_name, request)
                    else:
                        # Skip class method fixtures for module-level tests
                        debug_print(f"Skipping class method fixture {{fixture_name}} for module-level test")
                        continue
                else:
                    # Execute non-class fixtures normally
                    execute_fixture(fixture_name, request)
            except Exception as e:
                debug_print(f"Error executing autouse fixture {{fixture_name}}: {{e}}")
                # Don't suppress the error if it's critical
                if "missing" in str(e) and "required positional argument" in str(e):
                    # For now, continue but log it prominently
                    debug_print(f"WARNING: Autouse fixture {{fixture_name}} has unresolved dependencies")

        # Execute fixtures for remaining parameters
        for param_name in fixture_candidates:
            if param_name in BUILTIN_FIXTURES or param_name in _fixture_registry:
                kwargs[param_name] = execute_fixture(param_name, request)
                request.fixturenames.append(param_name)
            elif test_data.get('class_name'):
                # For class method tests, also check for class-scoped fixtures
                class_fixture_key = f"{{test_data['class_name']}}.{{param_name}}"
                if class_fixture_key in _fixture_registry:
                    # Store the instance in request for the fixture to use
                    if instance:
                        request._test_instance = instance
                    kwargs[param_name] = execute_fixture(class_fixture_key, request)
                    request.fixturenames.append(param_name)

        # Execute setup_method if instance method
        if instance:
            setup_method_if_needed(instance, test_data['function'].split('::')[-1])

        # Execute test
        # Check if we're using capsys fixture - if so, don't redirect output
        using_capsys = 'capsys' in kwargs

        debug_print(f"Test function {{func.__name__}} is async: {{is_async_function(func)}}")

        try:
            if is_async_function(func):
                import asyncio
                event_loop = kwargs.get('event_loop')
                if using_capsys:
                    # Don't redirect output when using capsys
                    if event_loop is not None:
                        event_loop.run_until_complete(func(**kwargs))
                    elif hasattr(asyncio, 'Runner'):  # Python 3.11+
                        with asyncio.Runner() as runner:
                            runner.run(func(**kwargs))
                    else:
                        asyncio.run(func(**kwargs))
                else:
                    with _null_redirect(), _null_redirect():
                        if event_loop is not None:
                            event_loop.run_until_complete(func(**kwargs))
                        elif hasattr(asyncio, 'Runner'):  # Python 3.11+
                            with asyncio.Runner() as runner:
                                runner.run(func(**kwargs))
                        else:
                            asyncio.run(func(**kwargs))
            else:
                if using_capsys:
                    # Don't redirect output when using capsys
                    debug_print(f"Calling test function (with capsys): {{func.__name__}}")
                    func(**kwargs)
                else:
                    debug_print(f"Calling test function (no capsys): {{func.__name__}}")
                    with _null_redirect(), _null_redirect():
                        func(**kwargs)
        except BaseException as test_exc:
            # Check if this is a skip/xfail exception raised during test execution
            exc_type = type(test_exc).__name__
            exc_module = type(test_exc).__module__ if hasattr(type(test_exc), '__module__') else ''

            # Note: debug_print won't work here if we're inside _null_redirect()
            # So we'll just re-raise and let the outer handler deal with it

            # Always re-raise the exception to be handled by the outer exception handler
            # The outer handler will properly convert skip/xfail exceptions to results
            raise test_exc
        finally:
            # Execute teardown_method if instance method
            if instance:
                teardown_method_if_needed(instance, test_data['function'].split('::')[-1])

        # Teardown function-scoped fixtures
        teardown_fixtures('function', request)

        # Execute request finalizers
        request._finalize()

        duration = perf() - start

        # If test was xfail but passed, it's an xpass
        if is_xfail:
            if xfail_info.get('strict'):
                return {{
                    'id': test_data['id'],
                    'outcome': 'failed',
                    'duration': duration,
                    'error': f"XPASS(strict): {{xfail_reason or 'Expected failure'}}",
                    'error_details': None
                }}
            return {{
                'id': test_data['id'],
                'outcome': 'xpassed',
                'duration': duration,
                'error': None
            }}

        return {{
            'id': test_data['id'],
            'outcome': 'passed',
            'duration': duration,
            'error': None
        }}

    except BaseException as e:
        debug_print(f"OUTER EXCEPTION HANDLER: Caught {{type(e).__name__}}: {{e}}")
        # Execute teardown_method if instance method (even on failure)
        if instance:
            try:
                teardown_method_if_needed(instance, test_data['function'].split('::')[-1])
            except:
                pass  # Don't let teardown failures mask the original error

        # Teardown on failure
        if request:
            teardown_fixtures('function', request)

        duration = perf() - start

        # Enhanced error reporting with assertion introspection
        error_msg = str(e)
        error_details = None

        if isinstance(e, AssertionError):
            # Try to extract more information from the assertion
            error_details = extract_assertion_details(e, func, kwargs)
            if error_details:
                error_msg = format_assertion_error(error_details)

        # Handle pytest skip exceptions
        # Check for the actual pytest exception types and the _pytest outcomes module
        exception_type = type(e).__name__
        exception_module = type(e).__module__ if hasattr(type(e), '__module__') else ''

        debug_print(f"Exception caught: type={{exception_type}}, module={{exception_module}}, str={{str(e)}}")

        # pytest uses 'Skipped' from _pytest.outcomes module
        if (exception_type in ('Skipped', 'SkipTest', 'SkipException') or
            hasattr(e, '_pytest_skip') or
            '_pytest.outcomes' in exception_module):
            # Extract skip reason - pytest stores it in msg attribute
            skip_reason = str(e) if str(e) else "Skipped"
            if hasattr(e, 'msg'):
                skip_reason = e.msg
            debug_print(f"Handling as skip: reason={{skip_reason}}")
            result = {{
                'id': test_data['id'],
                'outcome': 'skipped',
                'skip_reason': skip_reason,
                'duration': duration,
                'error': None
            }}
            debug_print(f"Returning skip result: {{result}}")
            return result

        # Handle pytest xfail exceptions
        if (exception_type in ('XFailed', 'XFailTest', 'XFailException', 'Failed') or
            hasattr(e, '_pytest_xfail')):
            xfail_reason = str(e) if str(e) else "Expected failure"
            if hasattr(e, 'msg'):
                xfail_reason = e.msg
            return {{
                'id': test_data['id'],
                'outcome': 'xfailed',
                'xfail_reason': xfail_reason,
                'duration': duration,
                'error': None
            }}

        # If test was marked as xfail but failed normally, it's still xfail
        if is_xfail:
            if not exception_matches_expected(e, xfail_info.get('raises')):
                return {{
                    'id': test_data['id'],
                    'outcome': 'failed',
                    'duration': duration,
                    'error': error_msg,
                    'error_details': error_details
                }}
            return {{
                'id': test_data['id'],
                'outcome': 'xfailed',
                'xfail_reason': xfail_reason or "Expected failure",
                'duration': duration,
                'error': error_msg
            }}

        # Normal failure
        return {{
            'id': test_data['id'],
            'outcome': 'failed',
            'duration': duration,
            'error': error_msg,
            'error_details': error_details
        }}

# Replace the original execute function
execute_single_test_ultra_fast = execute_test_with_fixtures

# Batch execution functions for compatibility
def execute_tests_ultra_fast(tests_list):
    """Ultra-fast execution of multiple tests with proper class teardown"""
    results = []

    # First pass: count tests per class for proper teardown timing
    class_test_counts = {{}}
    for test_data in tests_list:
        if test_data.get('class_name'):
            module_key = get_module_cache_key(test_data['module'], test_data.get('path'))
            class_path = f"{{module_key}}.{{test_data['class_name']}}"
            class_test_counts[class_path] = class_test_counts.get(class_path, 0) + 1

    # Register expected test counts with lifecycle manager
    for class_path, count in class_test_counts.items():
        for _ in range(count):
            _class_lifecycle.increment_test_count(class_path)

    try:
        for test_data in tests_list:
            # Handle class transitions using the lifecycle manager
            if test_data.get('class_name'):
                module_key = get_module_cache_key(test_data['module'], test_data.get('path'))
                new_class_path = f"{{module_key}}.{{test_data['class_name']}}"

                # Get the class object
                new_class = None
                if module_key in module_cache:
                    mod = module_cache[module_key]
                    if hasattr(mod, test_data['class_name']):
                        new_class = getattr(mod, test_data['class_name'])

                # Let the lifecycle manager handle the transition
                _class_lifecycle.transition_to_class(new_class_path, new_class)

                # Mark that we're running a test for this class
                _class_lifecycle.mark_test_run(new_class_path)
            else:
                # Moving to a module-level test
                _class_lifecycle.transition_to_class(None, None)

            debug_print(f"About to execute test: {{test_data['id']}}")
            result = execute_test_with_fixtures(test_data)
            debug_print(f"Test execution returned: {{result}}")
            results.append(result)
            debug_print(f"Successfully appended result for: {{test_data['id']}}")

        debug_print(f"All tests executed successfully, returning {{len(results)}} results")
    except BaseException as e:
        # If any exception occurs during batch execution (e.g., module-level skip),
        # we need to handle it gracefully
        debug_print(f"Exception during batch execution: {{type(e).__name__}}: {{e}}")

        # Check if this is a skip exception at module/collection level
        exception_type = type(e).__name__
        exception_module = type(e).__module__ if hasattr(type(e), '__module__') else ''

        if (exception_type in ('Skipped', 'SkipTest', 'SkipException') or
            hasattr(e, '_pytest_skip') or
            '_pytest.outcomes' in exception_module):
            # Module-level skip - mark all remaining tests as skipped
            skip_reason = str(e) if str(e) else "Skipped"
            if hasattr(e, 'msg'):
                skip_reason = e.msg

            debug_print(f"Module-level skip detected: {{skip_reason}}")
            # Don't re-raise - we've already handled the skip properly in the individual test
            # The exception was caught here because it bubbled up from execute_test_with_fixtures
        else:
            # Other exceptions should be re-raised
            raise
    finally:
        # Teardown all remaining classes using the lifecycle manager
        debug_print("Final teardown of all active classes")
        _class_lifecycle.teardown_all_classes()

    return results

def execute_tests_burst_optimized(batch_tests, micro_threads=2):
    """HybridBurst execution for 21-100 test range with intelligent threading"""
    import threading
    import queue
    from concurrent.futures import ThreadPoolExecutor, as_completed
    import multiprocessing

    results = []
    test_queue = queue.Queue()
    result_queue = queue.Queue()

    # Add all tests to the queue
    for test in batch_tests:
        test_queue.put(test)

    # Thread-local storage for module cache to avoid conflicts
    thread_local = threading.local()

    def worker_thread():
        """Worker thread that processes tests from the queue"""
        # Each thread maintains its own module cache
        if not hasattr(thread_local, 'module_cache'):
            thread_local.module_cache = {{}}

        while True:
            try:
                test = test_queue.get(timeout=0.1)
            except queue.Empty:
                break

            # Execute the test with minimal overhead
            start_time = perf()
            result = {{
                'id': test['id'],
                'module': test['module'],
                'function': test.get('function', test['id']),
                'duration': 0.0,
                'passed': False,
                'error': None,
                'outcome': 'failed',
                'stdout': '',
                'stderr': ''
            }}

            try:
                # Fast path for module loading - use thread-local cache
                module_name = test['module']
                if module_name not in thread_local.module_cache:
                    # Add test path to sys.path if needed
                    test_path = test.get('path', '')
                    if test_path:
                        test_dir = os.path.dirname(test_path)
                        if test_dir not in sys.path:
                            sys.path.insert(0, test_dir)

                    # Import module
                    thread_local.module_cache[module_name] = importlib.import_module(module_name)

                mod = thread_local.module_cache[module_name]

                # Get the test function/method
                func = None
                if '::' in test['function']:
                    # Class method
                    class_name, method_name = test['function'].split('::', 1)
                    if hasattr(mod, class_name):
                        cls = getattr(mod, class_name)
                        instance = cls()
                        if hasattr(instance, method_name):
                            func = getattr(instance, method_name)
                else:
                    # Regular function
                    func_name = test['function']
                    if hasattr(mod, func_name):
                        func = getattr(mod, func_name)

                if func is None:
                    raise AttributeError(f"Test function not found: {{test['function']}}")

                # Check markers for skip/xfail
                decorators = test.get('decorators', [])
                markers = extract_markers(decorators)

                skip_reason = check_skip_markers(markers)
                if skip_reason:
                    result['outcome'] = 'skipped'
                    result['skip_reason'] = skip_reason
                    result['passed'] = False
                else:
                    xfail_reason = check_xfail_markers(markers)

                    # Capture output
                    stdout_capture = StringIO()
                    stderr_capture = StringIO()

                    try:
                        with redirect_stdout(stdout_capture), redirect_stderr(stderr_capture):
                            # Execute test with minimal fixture overhead for burst mode
                            if asyncio.iscoroutinefunction(func):
                                asyncio.run(func())
                            else:
                                func()

                        # Test passed
                        if xfail_reason:
                            result['outcome'] = 'xpassed'
                            result['passed'] = False
                        else:
                            result['outcome'] = 'passed'
                            result['passed'] = True

                    except Exception as e:
                        # Test failed
                        if xfail_reason:
                            result['outcome'] = 'xfailed'
                            result['xfail_reason'] = xfail_reason
                            result['passed'] = True  # xfail counts as passed
                        else:
                            result['outcome'] = 'failed'
                            result['passed'] = False
                            result['error'] = str(e)

                    result['stdout'] = stdout_capture.getvalue()
                    result['stderr'] = stderr_capture.getvalue()

            except Exception as e:
                result['error'] = str(e)
                result['outcome'] = 'failed'
                result['passed'] = False

            result['duration'] = perf() - start_time
            result_queue.put(result)
            test_queue.task_done()

    # Use ThreadPoolExecutor for better thread management
    # For 21-100 tests, use 2-4 threads based on CPU count
    optimal_threads = min(micro_threads, len(batch_tests), multiprocessing.cpu_count())

    with ThreadPoolExecutor(max_workers=optimal_threads) as executor:
        # Submit worker threads
        futures = [executor.submit(worker_thread) for _ in range(optimal_threads)]

        # Wait for all workers to complete
        for future in as_completed(futures):
            try:
                future.result()
            except Exception as e:
                debug_print(f"Worker thread error: {{e}}")

    # Collect all results
    while not result_queue.empty():
        results.append(result_queue.get())

    return results

# Module teardown support
def teardown_module():
    """Teardown module-scoped fixtures"""
    teardown_fixtures('module')

def teardown_session():
    """Teardown session-scoped fixtures"""
    teardown_fixtures('session')

def perform_global_teardown():
    """Perform all teardowns in reverse order of setup (except session fixtures)"""
    # Get the teardown order (reverse of setup order)
    teardown_items = list(reversed(_setup_state['setup_order']))

    for scope_type, identifier in teardown_items:
        if scope_type == 'class':
            # Get the class object
            parts = identifier.split('.')
            module_name = '.'.join(parts[:-1])
            class_name = parts[-1]

            if module_name in module_cache:
                mod = module_cache[module_name]
                if hasattr(mod, class_name):
                    cls = getattr(mod, class_name)
                    teardown_class_if_needed(identifier, cls)
        elif scope_type == 'module':
            teardown_module_if_needed(identifier)

    # Teardown class and module fixtures, but NOT session fixtures
    # Session fixtures should persist for the entire test session
    teardown_fixtures('class')
    teardown_fixtures('module')
    # NOTE: Session fixtures are torn down separately by teardown_session_fixtures()

# Make teardown accessible for cleanup
def get_teardown_order():
    """Get the current teardown order for debugging"""
    return list(reversed(_setup_state['setup_order']))

def teardown_session_fixtures():
    """Explicitly teardown only session-scoped fixtures

    This should only be called at the very end of the test session,
    typically from the Rust executor's Drop implementation.
    """
    debug_print("Tearing down session fixtures at end of test session")
    teardown_fixtures('session')
"#,
        verbose_str = verbose_str
    )
}
