import json, sys, time, traceback, importlib, importlib.util, io, os, asyncio, platform, tempfile, pathlib

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
        return __builtins__['abs'](actual - self.expected) <= self.abs_tol if isinstance(self.abs_tol, (int, float)) else False
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


def run_test(test_item):
    """Execute a single test item and return the result as a dict."""
    start = time.time()

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
        test_dir = os.path.dirname(os.path.abspath(test_item["path"]))
        if test_dir not in sys.path:
            sys.path.insert(0, test_dir)
        # Import module (validate name is a valid Python identifier)
        module_name = os.path.splitext(os.path.basename(test_item["path"]))[0]
        if not module_name.isidentifier():
            module_name = "".join(c if c.isalnum() or c == "_" else "_" for c in module_name)
            if not module_name or module_name[0].isdigit():
                module_name = "_" + module_name
        mod = importlib.import_module(module_name)
        importlib.reload(mod)  # Ensure fresh import
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
                    conftest_key = "conftest_" + cp.replace(os.sep, "_").replace(".", "_")
                    if conftest_key in sys.modules:
                        del sys.modules[conftest_key]
                    import pytest as _pt
                    _orig_fix = _pt.fixture
                    _pt.fixture = lambda f=None, **kw: f if f is not None else (lambda fn: fn)
                    spec = importlib.util.spec_from_file_location(conftest_key, cp)
                    cmod = importlib.util.module_from_spec(spec)
                    spec.loader.exec_module(cmod)
                    _pt.fixture = _orig_fix
                    if hasattr(cmod, dep):
                        fixture_func = getattr(cmod, dep)
                        # Pass already-resolved fixtures as kwargs to support dependencies
                        import inspect as _insp
                        _sig = _insp.signature(fixture_func)
                        _fix_args = {}
                        for _pname in _sig.parameters:
                            if _pname in fixture_kwargs:
                                _fix_args[_pname] = fixture_kwargs[_pname]
                        result = fixture_func(**_fix_args)
                        import inspect
                        if inspect.isgenerator(result):
                            try:
                                fixture_kwargs[dep] = next(result)
                                fixture_cleanups.append(lambda gen=result: next(gen, None))
                            except StopIteration:
                                fixture_kwargs[dep] = None
                        else:
                            fixture_kwargs[dep] = result
                        break  # Found the fixture, stop searching conftest files

        # Call setup_module if present and not already called for this module
        if hasattr(mod, 'setup_module') and not getattr(mod, '_fastest_setup_done', False):
            mod.setup_module()
            mod._fastest_setup_done = True

        # Execute test
        result = None
        try:
            if test_item.get("class_name"):
                cls = getattr(mod, test_item["class_name"])
                if hasattr(cls, "setup_class"):
                    cls.setup_class()
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
            else:
                func = getattr(mod, test_item["function_name"])
                call_kwargs = dict(fixture_kwargs)
                params = test_item.get("parameters")
                if params and params.get("values"):
                    call_kwargs.update(params["values"])
                if call_kwargs:
                    result = func(**call_kwargs)
                else:
                    result = func()
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
        print(json.dumps(result), flush=True)
    except Exception as e:
        print(json.dumps({"error": str(e)}), flush=True)
