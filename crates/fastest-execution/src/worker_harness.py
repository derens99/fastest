import json, sys, time, traceback, importlib, importlib.util, io, os, asyncio, platform, tempfile, pathlib, inspect

# Ensure UTF-8 encoding for stdin/stdout on Windows (default may be cp1252)
if hasattr(sys.stdin, 'reconfigure'):
    sys.stdin.reconfigure(encoding='utf-8')
if hasattr(sys.stdout, 'reconfigure'):
    sys.stdout.reconfigure(encoding='utf-8')

# Add project rootdir to sys.path so package-relative imports work
# (e.g. `from tests.concurrency import sleep` in httpx's conftest.py)
_fastest_rootdir = os.environ.get('FASTEST_ROOTDIR', '')
if _fastest_rootdir and _fastest_rootdir not in sys.path:
    sys.path.insert(0, _fastest_rootdir)

# Cache the builtin abs() to avoid issues when __builtins__ is a module vs dict
builtins_abs = abs

# Worker-level caches (persist across test runs within a single worker process)
_module_cache = {}         # abs_path -> module object
_conftest_cache = {}       # conftest_abs_path -> module object
_setup_module_done = set() # abs paths where setup_module has been called
_setup_class_done = set()  # (abs_path, class_name) tuples where setup_class has been called

# Provide a minimal pytest compatibility shim so tests using pytest.raises/pytest.mark etc. work
class _PytestRaisesContext:
    """Context manager for pytest.raises."""
    def __init__(self, expected_exception, match=None):
        self.expected_exception = expected_exception
        self.match = match
        self.value = None
    def __enter__(self):
        return self
    def __exit__(self, exc_type, exc_val, exc_tb):
        if exc_type is None:
            raise AssertionError(f"DID NOT RAISE {self.expected_exception}")
        if not issubclass(exc_type, self.expected_exception):
            return False  # Re-raise
        self.value = exc_val
        if self.match:
            import re
            if not re.search(self.match, str(exc_val)):
                raise AssertionError(f"{exc_val!r} does not match {self.match!r}")
        return True  # Suppress the exception

class _PytestWarnsContext:
    """Context manager for pytest.warns."""
    def __init__(self, expected_warning=None, match=None):
        self.expected_warning = expected_warning
        self.match = match
    def __enter__(self):
        import warnings
        self._catch = warnings.catch_warnings(record=True)
        self._warnings = self._catch.__enter__()
        return self._warnings
    def __exit__(self, *args):
        self._catch.__exit__(*args)

class _PytestApprox:
    """Approximate equality for floating point comparisons."""
    def __init__(self, expected, rel=None, abs=None):
        self.expected = expected
        self.rel = rel
        self.abs_tol = abs if abs is not None else 1e-6
    def __eq__(self, actual):
        try:
            return builtins_abs(actual - self.expected) <= self.abs_tol if isinstance(self.abs_tol, (int, float)) else False
        except (TypeError, ValueError):
            return False
    def __ne__(self, actual):
        return not self.__eq__(actual)
    def __repr__(self):
        return f"approx({self.expected})"

class _PytestShim:
    """Minimal pytest shim providing raises, warns, approx, mark, fixture, param."""
    class _MarkNamespace:
        def __getattr__(self, name):
            def decorator(*args, **kwargs):
                if args and callable(args[0]):
                    return args[0]
                return lambda f: f
            return decorator
    mark = _MarkNamespace()
    def raises(self, expected_exception, *args, match=None, **kwargs):
        return _PytestRaisesContext(expected_exception, match=match)
    def warns(self, expected_warning=None, *args, match=None, **kwargs):
        return _PytestWarnsContext(expected_warning, match=match)
    def approx(self, expected, rel=None, abs=None):
        return _PytestApprox(expected, rel=rel, abs=abs)
    def fixture(self, func=None, **kwargs):
        if func is not None:
            return func
        return lambda f: f
    def param(self, *values, id=None, marks=()):
        return values if len(values) != 1 else values[0]
    def skip(self, reason=""):
        raise Exception(f"SKIPPED: {reason}")
    def fail(self, reason=""):
        raise AssertionError(reason)
    def importorskip(self, modname, minversion=None, reason=None):
        try:
            mod = importlib.import_module(modname)
            if minversion:
                ver = getattr(mod, '__version__', '')
                if ver < minversion:
                    raise Exception(f"SKIPPED: {reason or f'{modname}>={minversion} required'}")
            return mod
        except ImportError:
            raise Exception(f"SKIPPED: {reason or f'could not import {modname!r}'}")

# Install the shim as 'pytest' if the real package isn't available
try:
    import pytest as _real_pytest
    # Real pytest is available — patch in raises/warns/approx if missing
    if not hasattr(_real_pytest, 'raises'):
        _shim = _PytestShim()
        _real_pytest.raises = _shim.raises
except ImportError:
    sys.modules['pytest'] = _PytestShim()

_fixture_scope_cache = {'session': {}, 'module': {}, 'class': {}}
_current_module_path = None
_current_class_name = None


def _run_pending_teardowns():
    """Run teardown_class and teardown_module for the last processed class/module."""
    global _current_module_path, _current_class_name
    if _current_class_name is not None and _current_module_path and _current_module_path in _module_cache:
        mod = _module_cache[_current_module_path]
        if hasattr(mod, _current_class_name):
            cls = getattr(mod, _current_class_name)
            if hasattr(cls, 'teardown_class'):
                try:
                    cls.teardown_class()
                except Exception:
                    pass
    if _current_module_path and _current_module_path in _module_cache:
        mod = _module_cache[_current_module_path]
        if hasattr(mod, 'teardown_module'):
            try:
                _call_module_func(mod.teardown_module, mod)
            except Exception:
                pass
    _current_module_path = None
    _current_class_name = None


def _call_module_func(func, mod):
    """Call setup_module/teardown_module, passing the module if the function expects an argument."""
    try:
        sig = inspect.signature(func)
        if sig.parameters:
            func(mod)
        else:
            func()
    except (ValueError, TypeError):
        # Fallback: try with arg first, then without
        try:
            func(mod)
        except TypeError:
            func()


def _resolve_conftest_fixture(name, cmod, conftest_paths, fixture_kwargs, fixture_cleanups, _resolving=None):
    """Recursively resolve a conftest fixture and its dependencies.

    Handles session/module/class scope caching and yield fixtures.
    Recursively resolves any fixture dependencies before calling the fixture function.
    """
    if _resolving is None:
        _resolving = set()
    # Prevent infinite recursion
    if name in _resolving or name in fixture_kwargs:
        return
    _resolving.add(name)

    # Check scope cache first
    for sc in ('session', 'module', 'class'):
        if name in _fixture_scope_cache.get(sc, {}):
            fixture_kwargs[name] = _fixture_scope_cache[sc][name]
            return

    fixture_func = getattr(cmod, name, None)
    if fixture_func is None:
        # Search other conftest modules
        for cp in conftest_paths:
            abs_cp = os.path.abspath(cp)
            if abs_cp in _conftest_cache and hasattr(_conftest_cache[abs_cp], name):
                fixture_func = getattr(_conftest_cache[abs_cp], name)
                cmod = _conftest_cache[abs_cp]
                break
        if fixture_func is None:
            return

    fix_scope = getattr(fixture_func, '_fastest_scope', 'function') if fixture_func else 'function'

    # Re-check scope cache (may have been populated by dependency resolution)
    if fix_scope in ('session', 'module', 'class') and name in _fixture_scope_cache.get(fix_scope, {}):
        fixture_kwargs[name] = _fixture_scope_cache[fix_scope][name]
        return

    # Resolve dependencies first (recursive)
    sig = inspect.signature(fixture_func)
    for pname in sig.parameters:
        if pname not in fixture_kwargs and pname != 'self':
            # Try to find this dependency in conftest modules
            for cp in conftest_paths:
                abs_cp = os.path.abspath(cp)
                if abs_cp in _conftest_cache and hasattr(_conftest_cache[abs_cp], pname):
                    _resolve_conftest_fixture(pname, _conftest_cache[abs_cp], conftest_paths,
                                             fixture_kwargs, fixture_cleanups, _resolving)
                    break

    # Build kwargs from resolved fixtures
    fix_args = {}
    for pname in sig.parameters:
        if pname in fixture_kwargs:
            fix_args[pname] = fixture_kwargs[pname]

    result = fixture_func(**fix_args)
    if inspect.isgenerator(result):
        try:
            fixture_kwargs[name] = next(result)
            fixture_cleanups.append(lambda gen=result: next(gen, None))
        except StopIteration:
            fixture_kwargs[name] = None
    else:
        fixture_kwargs[name] = result

    # Store in scope cache if scoped
    if fix_scope in ('session', 'module', 'class'):
        _fixture_scope_cache[fix_scope][name] = fixture_kwargs[name]


def run_test(test_item):
    """Execute a single test item and return the result as a dict."""
    global _current_module_path, _current_class_name
    start = time.time()

    # Use absolute paths for consistent caching
    new_module = os.path.abspath(test_item.get("path", ""))
    new_class = test_item.get("class_name")

    # Module transition: run teardowns for previous module/class
    if new_module != _current_module_path:
        if _current_class_name is not None and _current_module_path and _current_module_path in _module_cache:
            prev_mod = _module_cache[_current_module_path]
            if hasattr(prev_mod, _current_class_name):
                prev_cls = getattr(prev_mod, _current_class_name)
                if hasattr(prev_cls, 'teardown_class'):
                    try: prev_cls.teardown_class()
                    except Exception: pass
        if _current_module_path and _current_module_path in _module_cache:
            prev_mod = _module_cache[_current_module_path]
            if hasattr(prev_mod, 'teardown_module'):
                try: _call_module_func(prev_mod.teardown_module, prev_mod)
                except Exception: pass
        _fixture_scope_cache['module'].clear()
        _fixture_scope_cache['class'].clear()
        _current_module_path = new_module
        _current_class_name = None

    # Class transition: run teardown_class for previous class
    if new_class != _current_class_name:
        if _current_class_name is not None and _current_module_path in _module_cache:
            prev_mod = _module_cache[_current_module_path]
            if hasattr(prev_mod, _current_class_name):
                prev_cls = getattr(prev_mod, _current_class_name)
                if hasattr(prev_cls, 'teardown_class'):
                    try: prev_cls.teardown_class()
                    except Exception: pass
        _fixture_scope_cache['class'].clear()
        _current_class_name = new_class

    # Check for skip/skipif markers before execution
    markers = test_item.get("markers") or []
    for m in markers:
        if m["name"] == "skip":
            reason = m.get("kwargs", {}).get("reason")
            if reason is None:
                args = m.get("args", [])
                reason = args[0] if args else None
            return {
                "test_id": test_item["id"],
                "outcome": "Skipped",
                "duration_ms": 0,
                "error": None,
                "stdout": "",
                "stderr": "",
                "reason": str(reason) if reason else None,
            }
        if m["name"] == "skipif":
            condition = m.get("args", [""])[0] if m.get("args") else ""
            reason = m.get("kwargs", {}).get("reason")
            try:
                if eval(str(condition)):
                    return {
                        "test_id": test_item["id"],
                        "outcome": "Skipped",
                        "duration_ms": 0,
                        "error": None,
                        "stdout": "",
                        "stderr": "",
                        "reason": str(reason) if reason else None,
                    }
            except Exception:
                pass  # If condition can't be evaluated, don't skip

    # Determine if test is marked xfail
    xfail_marker = None
    for m in markers:
        if m["name"] == "xfail":
            xfail_marker = m
            break
    xfail_reason = None
    xfail_strict = False
    if xfail_marker is not None:
        xfail_reason = xfail_marker.get("kwargs", {}).get("reason")
        if xfail_reason is None:
            args = xfail_marker.get("args", [])
            xfail_reason = args[0] if args else None
        xfail_strict = bool(xfail_marker.get("kwargs", {}).get("strict", False))

    stdout_capture = io.StringIO()
    stderr_capture = io.StringIO()
    old_stdout, old_stderr = sys.stdout, sys.stderr
    try:
        sys.stdout, sys.stderr = stdout_capture, stderr_capture
        # Add test dir to path
        test_dir = os.path.dirname(new_module)
        if test_dir not in sys.path:
            sys.path.insert(0, test_dir)
        # Compute package-qualified module name for relative imports.
        # Walk from the test file up to rootdir, collecting __init__.py-containing
        # directories to form the dotted package path (e.g. "tests.client.test_auth").
        module_name = os.path.splitext(os.path.basename(new_module))[0]
        package_name = None
        rootdir = os.environ.get('FASTEST_ROOTDIR', '')
        if rootdir:
            try:
                rel = os.path.relpath(new_module, rootdir)
                parts = rel.replace(os.sep, '/').split('/')
                # Remove .py extension from last part
                parts[-1] = os.path.splitext(parts[-1])[0]
                # Check if parent dirs have __init__.py (i.e. are packages)
                pkg_parts = []
                cur = rootdir
                is_package = True
                for part in parts[:-1]:
                    cur = os.path.join(cur, part)
                    if os.path.exists(os.path.join(cur, '__init__.py')):
                        pkg_parts.append(part)
                    else:
                        is_package = False
                        break
                if is_package and pkg_parts:
                    dotted = '.'.join(pkg_parts + [parts[-1]])
                    package_name = '.'.join(pkg_parts)
                    module_name = dotted
                    # Ensure parent packages are in sys.modules so relative imports work.
                    # E.g. for "tests.client.test_auth", register "tests" and "tests.client".
                    for i in range(len(pkg_parts)):
                        pkg_dotted = '.'.join(pkg_parts[:i+1])
                        if pkg_dotted not in sys.modules:
                            pkg_dir = os.path.join(rootdir, *pkg_parts[:i+1])
                            pkg_init = os.path.join(pkg_dir, '__init__.py')
                            try:
                                pkg_spec = importlib.util.spec_from_file_location(
                                    pkg_dotted, pkg_init,
                                    submodule_search_locations=[pkg_dir])
                                pkg_mod = importlib.util.module_from_spec(pkg_spec)
                                pkg_mod.__package__ = pkg_dotted
                                pkg_mod.__path__ = [pkg_dir]
                                sys.modules[pkg_dotted] = pkg_mod
                                pkg_spec.loader.exec_module(pkg_mod)
                            except Exception:
                                # If __init__.py can't be loaded, create a namespace package
                                import types
                                pkg_mod = types.ModuleType(pkg_dotted)
                                pkg_mod.__package__ = pkg_dotted
                                pkg_mod.__path__ = [pkg_dir]
                                sys.modules[pkg_dotted] = pkg_mod
            except (ValueError, IndexError):
                pass
        if not module_name.isidentifier() and '.' not in module_name:
            module_name = "".join(c if c.isalnum() or c == "_" else "_" for c in module_name)
            if not module_name or module_name[0].isdigit():
                module_name = "_" + module_name
        # Load module using cache (avoid unnecessary reimport)
        if new_module in _module_cache:
            mod = _module_cache[new_module]
        else:
            spec = importlib.util.spec_from_file_location(module_name, new_module,
                submodule_search_locations=[test_dir] if package_name else None)
            mod = importlib.util.module_from_spec(spec)
            if package_name:
                mod.__package__ = package_name
            sys.modules[module_name] = mod
            spec.loader.exec_module(mod)
            _module_cache[new_module] = mod
        # Get and call test function
        # Setup builtin fixtures
        fixture_deps = test_item.get("fixture_deps") or []
        fixture_kwargs = {}
        fixture_cleanups = []

        for dep in fixture_deps:
            if dep == "self":
                continue
            # Check if it's a parametrize param (already in params)
            params = test_item.get("parameters")
            if params and dep in (params.get("names") or []):
                continue
            if dep == "tmp_path":

                fixture_kwargs["tmp_path"] = pathlib.Path(tempfile.mkdtemp())
            elif dep == "capsys":
                _old_stdout_fix, _old_stderr_fix = sys.stdout, sys.stderr
                sys.stdout, sys.stderr = io.StringIO(), io.StringIO()
                class _CapturedOutput:
                    def __init__(self): self.out = ''; self.err = ''
                class _Capsys:
                    def readouterr(self_inner):
                        out = sys.stdout.getvalue() if hasattr(sys.stdout, 'getvalue') else ''
                        err = sys.stderr.getvalue() if hasattr(sys.stderr, 'getvalue') else ''
                        sys.stdout, sys.stderr = io.StringIO(), io.StringIO()
                        r = _CapturedOutput(); r.out = out; r.err = err; return r
                    def _restore(self_inner):
                        sys.stdout, sys.stderr = _old_stdout_fix, _old_stderr_fix
                fixture_kwargs["capsys"] = _Capsys()
                fixture_cleanups.append(lambda: fixture_kwargs["capsys"]._restore())
            elif dep == "capfd":
                class _CapturedFdOutput:
                    def __init__(self): self.out = ''; self.err = ''
                class _Capfd:
                    def __init__(self_inner):
                        self_inner._old_stdout_fd = os.dup(1)
                        self_inner._old_stderr_fd = os.dup(2)
                        self_inner._stdout_tmp = tempfile.TemporaryFile(mode='w+b')
                        self_inner._stderr_tmp = tempfile.TemporaryFile(mode='w+b')
                        os.dup2(self_inner._stdout_tmp.fileno(), 1)
                        os.dup2(self_inner._stderr_tmp.fileno(), 2)
                    def readouterr(self_inner):
                        sys.stdout.flush()
                        sys.stderr.flush()
                        try:
                            os.fsync(1)
                            os.fsync(2)
                        except OSError:
                            pass
                        self_inner._stdout_tmp.seek(0)
                        self_inner._stderr_tmp.seek(0)
                        out = self_inner._stdout_tmp.read().decode('utf-8', errors='replace')
                        err = self_inner._stderr_tmp.read().decode('utf-8', errors='replace')
                        self_inner._stdout_tmp.seek(0); self_inner._stdout_tmp.truncate()
                        self_inner._stderr_tmp.seek(0); self_inner._stderr_tmp.truncate()
                        r = _CapturedFdOutput(); r.out = out; r.err = err; return r
                    def _restore(self_inner):
                        os.dup2(self_inner._old_stdout_fd, 1)
                        os.dup2(self_inner._old_stderr_fd, 2)
                        os.close(self_inner._old_stdout_fd)
                        os.close(self_inner._old_stderr_fd)
                        self_inner._stdout_tmp.close()
                        self_inner._stderr_tmp.close()
                fixture_kwargs["capfd"] = _Capfd()
                fixture_cleanups.append(lambda: fixture_kwargs["capfd"]._restore())
            elif dep == "monkeypatch":
                class _MonkeyPatch:
                    def __init__(self_inner):
                        self_inner._patches = []
                        self_inner._env_patches = []
                    _NOTSET = object()
                    def setattr(self_inner, target, name=_NOTSET, value=_NOTSET):
                        if value is self_inner._NOTSET:
                            if name is self_inner._NOTSET:
                                raise TypeError("setattr requires at least 2 arguments")
                            value = name
                            # target is a dotted string like "os.path.exists"
                            parts = target.rsplit('.', 1)
                            if len(parts) == 2:
                                modpath, attrname = parts
                                # Walk the dotted path: import module, then getattr for subattrs
                                segs = modpath.split('.')
                                obj = importlib.import_module(segs[0])
                                for seg in segs[1:]:
                                    obj = getattr(obj, seg)
                                target = obj
                                name = attrname
                            else:
                                raise TypeError("string target must be dotted path")
                        old = getattr(target, name)
                        self_inner._patches.append((target, name, old))
                        setattr(target, name, value)
                    def delattr(self_inner, target, name):
                        old = getattr(target, name)
                        self_inner._patches.append((target, name, old))
                        delattr(target, name)
                    def setenv(self_inner, key, value):
                        old = os.environ.get(key)
                        self_inner._env_patches.append((key, old))
                        os.environ[key] = value
                    def delenv(self_inner, key, raising=True):
                        old = os.environ.get(key)
                        self_inner._env_patches.append((key, old))
                        if key in os.environ: del os.environ[key]
                        elif raising: raise KeyError(key)
                    def chdir(self_inner, path):
                        old = os.getcwd()
                        self_inner._patches.append(('__cwd__', '__cwd__', old))
                        os.chdir(str(path))
                    def syspath_prepend(self_inner, path):
                        sys.path.insert(0, str(path))
                        self_inner._patches.append(('__syspath__', str(path), None))
                    def setitem(self_inner, mapping, key, value):
                        old = mapping.get(key, self_inner._NOTSET)
                        self_inner._patches.append(('__item__', (mapping, key), old))
                        mapping[key] = value
                    def delitem(self_inner, mapping, key, raising=True):
                        old = mapping.get(key, self_inner._NOTSET)
                        self_inner._patches.append(('__item__', (mapping, key), old))
                        if key in mapping: del mapping[key]
                        elif raising: raise KeyError(key)
                    def context(self_inner):
                        import contextlib
                        @contextlib.contextmanager
                        def _ctx():
                            m = _MonkeyPatch()
                            try:
                                yield m
                            finally:
                                m.undo()
                        return _ctx()
                    def undo(self_inner):
                        for target, name, old in reversed(self_inner._patches):
                            if target == '__cwd__':
                                os.chdir(old)
                            elif target == '__syspath__':
                                try: sys.path.remove(name)
                                except ValueError: pass
                            elif target == '__item__':
                                mapping, key = name
                                if old is self_inner._NOTSET:
                                    mapping.pop(key, None)
                                else:
                                    mapping[key] = old
                            else:
                                setattr(target, name, old)
                        self_inner._patches.clear()
                        for key, old in reversed(self_inner._env_patches):
                            if old is None: os.environ.pop(key, None)
                            else: os.environ[key] = old
                        self_inner._env_patches.clear()
                fixture_kwargs["monkeypatch"] = _MonkeyPatch()
                fixture_cleanups.append(lambda: fixture_kwargs["monkeypatch"].undo())
            elif dep == "caplog":
                import logging as _logging
                class _LogCaptureHandler(_logging.Handler):
                    def __init__(self):
                        super().__init__()
                        self.records = []
                    def emit(self, record):
                        self.records.append(record)
                class _Caplog:
                    def __init__(self):
                        self.handler = _LogCaptureHandler()
                        self.handler.setLevel(_logging.DEBUG)
                        _logging.root.addHandler(self.handler)
                        self._initial_level = _logging.root.level
                        _logging.root.setLevel(_logging.DEBUG)
                    @property
                    def records(self):
                        return self.handler.records
                    @property
                    def text(self):
                        return '\n'.join(self.handler.format(r) for r in self.records)
                    @property
                    def messages(self):
                        return [r.getMessage() for r in self.records]
                    def set_level(self, level):
                        self.handler.setLevel(level)
                    def clear(self):
                        self.handler.records.clear()
                    def _restore(self):
                        _logging.root.removeHandler(self.handler)
                        _logging.root.setLevel(self._initial_level)
                fixture_kwargs["caplog"] = _Caplog()
                fixture_cleanups.append(lambda: fixture_kwargs["caplog"]._restore())
            elif dep == "request":
                class _FixtureRequest:
                    def __init__(self):
                        self.param = None
                        self.node = None
                        self.config = None
                        self.fspath = None
                        self._finalizers = []
                    def addfinalizer(self, func):
                        self._finalizers.append(func)
                    def _run_finalizers(self):
                        for func in reversed(self._finalizers):
                            func()
                        self._finalizers.clear()
                fixture_kwargs["request"] = _FixtureRequest()
                fixture_cleanups.append(lambda: fixture_kwargs["request"]._run_finalizers())
            else:
                # Check fixture scope cache before executing
                _scope_cache_hit = False
                for _sc_scope in ('session', 'module', 'class'):
                    if dep in _fixture_scope_cache.get(_sc_scope, {}):
                        fixture_kwargs[dep] = _fixture_scope_cache[_sc_scope][dep]
                        _scope_cache_hit = True
                        break
                if _scope_cache_hit:
                    continue
                # Try loading from conftest.py files (load entire hierarchy)
                conftest_paths = []
                conftest_dir = test_dir
                while conftest_dir:
                    cp = os.path.join(conftest_dir, "conftest.py")
                    if os.path.exists(cp):
                        conftest_paths.append(cp)
                    parent = os.path.dirname(conftest_dir)
                    if parent == conftest_dir:
                        break
                    conftest_dir = parent
                # Load from root to leaf so deeper conftest overrides
                conftest_paths.reverse()
                for cp in conftest_paths:
                    # Use conftest cache to avoid re-executing conftest.py on every lookup
                    abs_cp = os.path.abspath(cp)
                    if abs_cp in _conftest_cache:
                        cmod = _conftest_cache[abs_cp]
                    else:
                        conftest_key = "conftest_" + cp.replace(os.sep, "_").replace(".", "_")
                        if conftest_key in sys.modules:
                            del sys.modules[conftest_key]
                        import pytest as _pt
                        _orig_fix = _pt.fixture
                        # Replace pytest.fixture with a scope-preserving decorator
                        # so that _fastest_scope metadata is recorded on fixture functions.
                        # The old identity lambda stripped all metadata (scope, autouse, params).
                        def _scope_recording_fixture(f=None, *, scope='function', autouse=False, params=None, **kw):
                            if f is None:
                                return lambda fn: _scope_recording_fixture(fn, scope=scope, autouse=autouse, params=params, **kw)
                            f._fastest_scope = scope
                            f._fastest_autouse = autouse
                            f._fastest_params = params
                            return f
                        _pt.fixture = _scope_recording_fixture
                        spec = importlib.util.spec_from_file_location(conftest_key, cp)
                        cmod = importlib.util.module_from_spec(spec)
                        spec.loader.exec_module(cmod)
                        _pt.fixture = _orig_fix
                        _conftest_cache[abs_cp] = cmod
                    if hasattr(cmod, dep):
                        _resolve_conftest_fixture(dep, cmod, conftest_paths, fixture_kwargs, fixture_cleanups)
                        break  # Found the fixture, stop searching conftest files

        # Call setup_module if present and not already called for this module
        if hasattr(mod, 'setup_module') and new_module not in _setup_module_done:
            _setup_module_done.add(new_module)  # Mark before calling to avoid retry on error
            _call_module_func(mod.setup_module, mod)

        # Execute test
        result = None
        try:
            if test_item.get("class_name"):
                cls = getattr(mod, test_item["class_name"])
                cls_key = (new_module, test_item["class_name"])
                if hasattr(cls, "setup_class") and cls_key not in _setup_class_done:
                    cls.setup_class()
                    _setup_class_done.add(cls_key)
                instance = cls()
                try:
                    if hasattr(instance, "setup_method"):
                        instance.setup_method()
                    func = getattr(instance, test_item["function_name"])
                    # Merge parametrize params with fixture kwargs
                    call_kwargs = dict(fixture_kwargs)
                    params = test_item.get("parameters")
                    if params and params.get("values"):
                        call_kwargs.update(params["values"])
                    if call_kwargs:
                        result = func(**call_kwargs)
                    else:
                        result = func()
                finally:
                    if hasattr(instance, "teardown_method"):
                        instance.teardown_method()
                    # teardown_class is deferred to class/module transition
            else:
                func = getattr(mod, test_item["function_name"])
                if hasattr(mod, 'setup_function'):
                    mod.setup_function(func)
                try:
                    call_kwargs = dict(fixture_kwargs)
                    params = test_item.get("parameters")
                    if params and params.get("values"):
                        call_kwargs.update(params["values"])
                    if call_kwargs:
                        result = func(**call_kwargs)
                    else:
                        result = func()
                finally:
                    if hasattr(mod, 'teardown_function'):
                        mod.teardown_function(func)
        finally:
            for cleanup in fixture_cleanups:
                try:
                    cleanup()
                except Exception:
                    pass
        # If the test is async, the call returns a coroutine — run it
        if result is not None and asyncio.iscoroutine(result):
            asyncio.run(result)
        # Test passed — check if xfail (passed when expected to fail = XPassed)
        if xfail_marker is not None:
            if xfail_strict:
                return {
                    "test_id": test_item["id"],
                    "outcome": "Failed",
                    "duration_ms": int((time.time() - start) * 1000),
                    "error": "test unexpectedly passed (strict xfail)",
                    "stdout": stdout_capture.getvalue(),
                    "stderr": stderr_capture.getvalue(),
                    "reason": str(xfail_reason) if xfail_reason else None,
                }
            return {
                "test_id": test_item["id"],
                "outcome": "XPassed",
                "duration_ms": int((time.time() - start) * 1000),
                "error": None,
                "stdout": stdout_capture.getvalue(),
                "stderr": stderr_capture.getvalue(),
                "reason": str(xfail_reason) if xfail_reason else None,
            }
        return {
            "test_id": test_item["id"],
            "outcome": "Passed",
            "duration_ms": int((time.time() - start) * 1000),
            "error": None,
            "stdout": stdout_capture.getvalue(),
            "stderr": stderr_capture.getvalue(),
            "reason": None,
        }
    except Exception as e:
        # Check for runtime pytest.skip()
        error_str = str(e)
        if error_str.startswith("SKIPPED:") or "SKIPPED:" in traceback.format_exc():
            reason = error_str.replace("SKIPPED:", "").strip() if "SKIPPED:" in error_str else None
            return {
                "test_id": test_item["id"],
                "outcome": "Skipped",
                "duration_ms": int((time.time() - start) * 1000),
                "error": None,
                "stdout": stdout_capture.getvalue(),
                "stderr": stderr_capture.getvalue(),
                "reason": reason,
            }
        # Test failed — check if xfail (failed when expected to fail = XFailed)
        if xfail_marker is not None:
            return {
                "test_id": test_item["id"],
                "outcome": "XFailed",
                "duration_ms": int((time.time() - start) * 1000),
                "error": None,
                "stdout": stdout_capture.getvalue(),
                "stderr": stderr_capture.getvalue(),
                "reason": str(xfail_reason) if xfail_reason else None,
            }
        # Distinguish import/collection errors from assertion failures
        is_collection_error = isinstance(e, (ModuleNotFoundError, ImportError, SyntaxError))
        outcome = "Error" if is_collection_error else "Failed"
        return {
            "test_id": test_item["id"],
            "outcome": outcome,
            "duration_ms": int((time.time() - start) * 1000),
            "error": traceback.format_exc(),
            "stdout": stdout_capture.getvalue(),
            "stderr": stderr_capture.getvalue(),
            "reason": None,
        }
    finally:
        sys.stdout, sys.stderr = old_stdout, old_stderr


for line in sys.stdin:
    line = line.strip()
    if not line:
        continue  # Skip blank lines instead of breaking the worker loop
    if line == "EXIT":
        break
    try:
        result = run_test(json.loads(line))
        print("FASTEST_RESULT:" + json.dumps(result), flush=True)
    except Exception as e:
        print("FASTEST_RESULT:" + json.dumps({"error": str(e)}), flush=True)

# Run final teardowns for the last module/class processed
_run_pending_teardowns()
