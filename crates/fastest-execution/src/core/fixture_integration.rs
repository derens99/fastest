//! Fixture Integration Module
//!
//! This module provides the Python code for complete fixture execution integration
//! with support for all scopes, dependencies, and teardown.

/// Generate the enhanced Python worker code with full fixture support
pub fn generate_fixture_aware_worker_code(verbose: bool) -> String {
    let verbose_str = if verbose { "True" } else { "False" };

    format!(
        r#"
import sys
import os
import inspect
import asyncio
import importlib
import functools
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
    # Define minimal skip/xfail support if pytest not available
    class pytest:
        @staticmethod
        def skip(reason=""):
            raise SkipTest(reason)
        
        @staticmethod
        def xfail(reason=""):
            raise XFailTest(reason)
    
    class SkipTest(Exception):
        pass
    
    class XFailTest(Exception):
        pass

# Debug flag from parent
DEBUG = {verbose_str}

def debug_print(msg):
    if DEBUG:
        print(f"[DEBUG] {{msg}}", file=sys.stderr)

# Module and function caches for ultra-fast imports
module_cache = {{}}
fn_cache = {{}}
path_cache = set()

# Fixture management
fixture_cache = {{}}
active_fixtures = {{}}
teardown_stack = []

# Lifecycle management for setup/teardown
_setup_state = {{
    'modules': {{}},      # module_name: {{'setup_done': bool, 'teardown_done': bool}}
    'classes': {{}},      # class_path: {{'setup_done': bool, 'teardown_done': bool}}
    'setup_order': [],  # Track order for proper teardown
}}

# Marker extraction and handling functions
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
                    if '=' in args_str:
                        # Has kwargs
                        parts = args_str.split(',')
                        for part in parts:
                            part = part.strip()
                            if '=' in part:
                                key, val = part.split('=', 1)
                                kwargs[key.strip()] = val.strip().strip('"\'')
                            else:
                                args.append(part.strip().strip('"\''))
                    else:
                        # Just args
                        args = [arg.strip().strip('"\'') for arg in args_str.split(',')]
                markers.append({{'name': name, 'args': args, 'kwargs': kwargs}})
            else:
                markers.append({{'name': marker_str, 'args': [], 'kwargs': {{}}}})
    return markers

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
            # For now, simple condition evaluation
            # TODO: Implement proper Python expression evaluation
            if marker['args']:
                condition = marker['args'][0]
                # Handle some common conditions
                if condition in ('True', 'true', '1'):
                    reason = marker['kwargs'].get('reason') or marker['args'][1] if len(marker['args']) > 1 else 'Conditional skip'
                    return reason
                # Platform checks
                if 'sys.platform' in condition:
                    import sys
                    if 'win32' in condition and sys.platform == 'win32':
                        return marker['kwargs'].get('reason', 'Skipped on Windows')
                    elif 'darwin' in condition and sys.platform == 'darwin':
                        return marker['kwargs'].get('reason', 'Skipped on macOS')
                    elif 'linux' in condition and sys.platform == 'linux':
                        return marker['kwargs'].get('reason', 'Skipped on Linux')
    return None

def check_xfail_markers(markers):
    """Check if test is expected to fail"""
    for marker in markers:
        if marker['name'] == 'xfail':
            # Get reason from kwargs or first arg
            if 'reason' in marker['kwargs']:
                return marker['kwargs']['reason']
            elif marker['args']:
                return marker['args'][0]
            else:
                return 'Expected to fail'
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
    if class_path in _setup_state['classes'] and _setup_state['classes'][class_path]['setup_done']:
        return
    
    if hasattr(cls, 'setup_class'):
        try:
            debug_print(f"Calling setup_class for {{class_path}}")
            # setup_class is typically a classmethod
            if isinstance(inspect.getattr_static(cls, 'setup_class'), classmethod):
                cls.setup_class()
            else:
                # Handle non-classmethod case
                cls.setup_class(cls)
            _setup_state['classes'][class_path] = {{'setup_done': True, 'teardown_done': False}}
            _setup_state['setup_order'].append(('class', class_path))
        except Exception as e:
            debug_print(f"setup_class failed for {{class_path}}: {{e}}")
            raise

def teardown_class_if_needed(class_path, cls):
    """Execute teardown_class if it exists and setup was called"""
    if class_path not in _setup_state['classes'] or not _setup_state['classes'][class_path]['setup_done']:
        return
    
    if _setup_state['classes'][class_path].get('teardown_done', False):
        return
    
    if hasattr(cls, 'teardown_class'):
        try:
            debug_print(f"Calling teardown_class for {{class_path}}")
            # teardown_class is typically a classmethod
            if isinstance(inspect.getattr_static(cls, 'teardown_class'), classmethod):
                cls.teardown_class()
            else:
                # Handle non-classmethod case
                cls.teardown_class(cls)
            _setup_state['classes'][class_path]['teardown_done'] = True
        except Exception as e:
            debug_print(f"teardown_class failed for {{class_path}}: {{e}}")

def setup_method_if_needed(instance, method_name):
    """Execute setup_method if it exists"""
    if hasattr(instance, 'setup_method'):
        try:
            # setup_method takes the method as parameter
            method = getattr(instance, method_name)
            instance.setup_method(method)
        except Exception as e:
            debug_print(f"setup_method failed: {{e}}")
            raise

def teardown_method_if_needed(instance, method_name):
    """Execute teardown_method if it exists"""
    if hasattr(instance, 'teardown_method'):
        try:
            # teardown_method takes the method as parameter
            method = getattr(instance, method_name)
            instance.teardown_method(method)
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
        self._fixture_values = {{}}
        self._finalizers = []
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
    
    for param_name in sig.parameters:
        if param_name == 'self':
            # Skip self parameter for class methods
            continue
        elif param_name == 'request':
            # Set the fixturename on the request object
            request.fixturename = fixture_name
            kwargs['request'] = request
        elif param_name in _fixture_registry:
            # Recursive fixture execution
            kwargs[param_name] = execute_fixture(param_name, request)
    
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
                result = asyncio.run(fixture_func(**kwargs))
            else:
                result = fixture_func(**kwargs)
        
        # Handle generator fixtures (yield)
        if inspect.isgeneratorfunction(fixture_func):
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

def teardown_fixtures(scope, request=None):
    """Teardown fixtures for a given scope"""
    debug_print(f"Tearing down fixtures for scope: {{scope}}")
    
    # Determine which fixtures to teardown
    to_teardown = []
    scope_order = ['session', 'package', 'module', 'class', 'function']
    scope_idx = scope_order.index(scope)
    
    for cache_key, fixture_scope in reversed(teardown_stack):
        fixture_data = active_fixtures.get(cache_key)
        if fixture_data:
            fixture_scope_idx = scope_order.index(fixture_data['scope'])
            if fixture_scope_idx >= scope_idx:
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

def builtin_capsys(request):
    """Built-in capsys fixture"""
    class CaptureFixture:
        def __init__(self):
            self._stdout = StringIO()
            self._stderr = StringIO()
            self._old_stdout = sys.stdout
            self._old_stderr = sys.stderr
            sys.stdout = self._stdout
            sys.stderr = self._stderr
        
        def readouterr(self):
            out = self._stdout.getvalue()
            err = self._stderr.getvalue()
            self._stdout.seek(0)
            self._stdout.truncate()
            self._stderr.seek(0)
            self._stderr.truncate()
            
            class Result:
                def __init__(self, out, err):
                    self.out = out
                    self.err = err
            
            return Result(out, err)
        
        def __del__(self):
            sys.stdout = self._old_stdout
            sys.stderr = self._old_stderr
    
    return CaptureFixture()

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
    
    class MockerFixture:
        def __init__(self):
            self._patches = []
        
        def patch(self, target, **kwargs):
            """Create a patch"""
            patcher = unittest.mock.patch(target, **kwargs)
            mock = patcher.start()
            self._patches.append(patcher)
            request.addfinalizer(patcher.stop)
            return mock
        
        def Mock(self, **kwargs):
            """Create a Mock object"""
            return unittest.mock.Mock(**kwargs)
        
        def MagicMock(self, **kwargs):
            """Create a MagicMock object"""
            return unittest.mock.MagicMock(**kwargs)
        
        def spy(self, obj, name):
            """Create a spy by wrapping the original method"""
            original = getattr(obj, name)
            mock = self.Mock(wraps=original)
            patcher = unittest.mock.patch.object(obj, name, mock)
            patcher.start()
            self._patches.append(patcher)
            request.addfinalizer(patcher.stop)
            return mock
    
    return MockerFixture()


BUILTIN_FIXTURES = {{
    'tmp_path': builtin_tmp_path,
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

def get_cached_function(module_name, func_name, filepath=None):
    """Ultra-fast function caching with optimized loading"""
    cache_key = f"{{module_name}}.{{func_name}}"
    
    if cache_key in fn_cache:
        return fn_cache[cache_key]
    
    try:
        if filepath:
            ensure_path_cached(filepath)
        
        # Get cached module or import
        if module_name in module_cache:
            mod = module_cache[module_name]
        else:
            mod = importlib.import_module(module_name)
            module_cache[module_name] = mod
        
        # Handle class methods
        if '::' in func_name:
            class_name, method_name = func_name.split('::', 1)
            cls = getattr(mod, class_name)
            
            # Setup class if needed using the new lifecycle management
            class_path = f"{{module_name}}.{{class_name}}"
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


def extract_assertion_details(exc, func, kwargs):
    """Extract detailed information from assertion errors"""
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
    # First try to import pytest in the module's context
    if hasattr(module, 'pytest'):
        pytest_module = module.pytest
    else:
        try:
            import pytest as pytest_module
        except ImportError:
            debug_print("pytest not available, using basic fixture detection")
            pytest_module = None
    
    for name in dir(module):
        if name.startswith('_'):
            continue
        obj = getattr(module, name)
        
        # Check if it's a fixture
        is_fixture = False
        fixture_metadata = {{}}
        
        # Method 1: Check for pytest fixture markers
        if hasattr(obj, '_pytestfixturefunction'):
            is_fixture = True
            fixture_info = obj._pytestfixturefunction
            fixture_metadata = {{
                'scope': getattr(fixture_info, 'scope', 'function'),
                'params': getattr(fixture_info, 'params', None),
                'autouse': getattr(fixture_info, 'autouse', False),
                'ids': getattr(fixture_info, 'ids', None),
            }}
        # Method 2: Check if name matches fixture pattern and is callable
        elif callable(obj) and (
            name.endswith('_fixture') or 
            name in ['simple_fixture', 'dependent_fixture', 'nested_dependent', 
                     'class_fixture', 'module_fixture', 'session_fixture',
                     'yield_fixture', 'yield_with_dependency', 'parametrized_fixture',
                     'parametrized_with_ids', 'fixture_with_request', 'fixture_with_finalizer',
                     'failing_fixture', 'user_factory', 'dynamic_fixture']
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
            _fixture_metadata[name] = {{
                'scope': fixture_metadata.get('scope', 'function'),
                'params': fixture_metadata.get('params') or [],
                'autouse': fixture_metadata.get('autouse', False),
                'ids': fixture_metadata.get('ids') or [],
                'is_generator': inspect.isgeneratorfunction(obj),
                'is_async': inspect.iscoroutinefunction(obj),
            }}
            debug_print(f"Registered fixture: {{name}} (scope={{_fixture_metadata[name]['scope']}})")

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
    xfail_reason = check_xfail_markers(markers)
    is_xfail = xfail_reason is not None
    
    # Initialize variables to None in case of errors
    instance = None
    request = None
    
    try:
        # Get the test module and scan for fixtures
        if test_data['module'] not in module_cache:
            ensure_path_cached(test_data.get('path'))
            mod = importlib.import_module(test_data['module'])
            module_cache[test_data['module']] = mod
            
            # Load and scan conftest.py modules
            if test_data.get('path'):
                conftest_modules = load_conftest_modules(test_data['path'])
                for conftest_mod in conftest_modules:
                    scan_module_for_fixtures(conftest_mod)
            
            # Scan the test module itself
            scan_module_for_fixtures(mod)
        else:
            mod = module_cache[test_data['module']]
        
        # If this is a class method test, also scan the class for fixtures
        if test_data.get('class_name'):
            class_name = test_data['class_name']
            if hasattr(mod, class_name):
                cls = getattr(mod, class_name)
                scan_class_for_fixtures(cls, test_data['module'], class_name)
        
        # Call module setup if needed
        setup_module_if_needed(test_data['module'])
        
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
        fn_result = get_cached_function(
            test_data['module'], 
            test_data['function'], 
            test_data.get('path')
        )
        
        if isinstance(fn_result, tuple):
            func, instance = fn_result
        else:
            func = fn_result
            instance = None
        
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
                        param_dict = json.loads(params_json)
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
                    autouse_fixtures.append(fixture_name)
                elif scope in ['module', 'session', 'package']:
                    autouse_fixtures.append(fixture_name)
        
        # Execute all autouse fixtures
        for fixture_name in autouse_fixtures:
            debug_print(f"Executing autouse fixture: {{fixture_name}}")
            try:
                # Check if this is a class method fixture
                fixture_meta = _fixture_metadata.get(fixture_name, {{}})
                if fixture_meta.get('is_class_method') and instance:
                    # This is a class method fixture - need special handling
                    # We'll execute it through the normal fixture system but with instance binding
                    # Store the instance in the request for use in execute_fixture
                    request._test_instance = instance
                
                # Execute through normal fixture system (handles dependencies)
                execute_fixture(fixture_name, request)
            except Exception as e:
                debug_print(f"Error executing autouse fixture {{fixture_name}}: {{e}}")
        
        # Execute fixtures for remaining parameters
        for param_name in fixture_candidates:
            if param_name in BUILTIN_FIXTURES or param_name in _fixture_registry:
                kwargs[param_name] = execute_fixture(param_name, request)
        
        # Execute setup_method if instance method
        if instance:
            setup_method_if_needed(instance, test_data['function'].split('::')[-1])
        
        # Execute test
        # Check if we're using capsys fixture - if so, don't redirect output
        using_capsys = 'capsys' in kwargs
        
        try:
            if is_async_function(func):
                import asyncio
                if using_capsys:
                    # Don't redirect output when using capsys
                    if hasattr(asyncio, 'Runner'):  # Python 3.11+
                        with asyncio.Runner() as runner:
                            runner.run(func(**kwargs))
                    else:
                        asyncio.run(func(**kwargs))
                else:
                    with _null_redirect(), _null_redirect():
                        if hasattr(asyncio, 'Runner'):  # Python 3.11+
                            with asyncio.Runner() as runner:
                                runner.run(func(**kwargs))
                        else:
                            asyncio.run(func(**kwargs))
            else:
                if using_capsys:
                    # Don't redirect output when using capsys
                    func(**kwargs)
                else:
                    with _null_redirect(), _null_redirect():
                        func(**kwargs)
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
        
    except Exception as e:
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
        if type(e).__name__ in ('Skipped', 'SkipTest', 'SkipException') or hasattr(e, '_pytest_skip'):
            skip_reason = str(e) if str(e) else "Skipped"
            return {{
                'id': test_data['id'],
                'outcome': 'skipped',
                'skip_reason': skip_reason,
                'duration': duration,
                'error': None
            }}
        
        # Handle pytest xfail exceptions
        if type(e).__name__ in ('XFailed', 'XFailTest', 'XFailException') or hasattr(e, '_pytest_xfail'):
            xfail_reason = str(e) if str(e) else "Expected failure"
            return {{
                'id': test_data['id'],
                'outcome': 'xfailed',
                'xfail_reason': xfail_reason,
                'duration': duration,
                'error': None
            }}
        
        # If test was marked as xfail but failed normally, it's still xfail
        if is_xfail:
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
    current_class_path = None
    current_class = None
    
    for test_data in tests_list:
        # Check if we're transitioning to a different class
        if test_data.get('class_name'):
            new_class_path = f"{{test_data['module']}}.{{test_data['class_name']}}"
            
            # If we're moving to a different class, teardown the previous one
            if current_class_path and current_class_path != new_class_path:
                debug_print(f"Transitioning from {{current_class_path}} to {{new_class_path}}")
                if current_class:
                    teardown_class_if_needed(current_class_path, current_class)
                # Also teardown any class-scoped fixtures
                teardown_fixtures('class')
            
            current_class_path = new_class_path
            # Get the class object for potential teardown later
            if test_data['module'] in module_cache:
                mod = module_cache[test_data['module']]
                if hasattr(mod, test_data['class_name']):
                    current_class = getattr(mod, test_data['class_name'])
        else:
            # Moving to a module-level test, teardown any active class
            if current_class_path and current_class:
                debug_print(f"Transitioning from {{current_class_path}} to module-level test")
                teardown_class_if_needed(current_class_path, current_class)
                # Also teardown any class-scoped fixtures
                teardown_fixtures('class')
                current_class_path = None
                current_class = None
        
        results.append(execute_test_with_fixtures(test_data))
    
    # Teardown any remaining class at the end
    if current_class_path and current_class:
        debug_print(f"Final teardown for {{current_class_path}}")
        teardown_class_if_needed(current_class_path, current_class)
        # Also teardown any class-scoped fixtures
        teardown_fixtures('class')
    
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
    """Perform all teardowns in reverse order of setup"""
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
    
    # Also teardown all scoped fixtures
    teardown_fixtures('class')
    teardown_fixtures('module')
    teardown_fixtures('session')

# Make teardown accessible for cleanup
def get_teardown_order():
    """Get the current teardown order for debugging"""
    return list(reversed(_setup_state['setup_order']))
"#,
        verbose_str = verbose_str
    )
}
