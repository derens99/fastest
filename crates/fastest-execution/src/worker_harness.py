import json, sys, time, traceback, importlib, importlib.util, io, os, asyncio, platform, tempfile, pathlib


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
    if xfail_marker is not None:
        xfail_reason = xfail_marker.get("kwargs", {}).get("reason")
        if xfail_reason is None:
            args = xfail_marker.get("args", [])
            xfail_reason = args[0] if args else None

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
            if dep == "self" or dep == "request":
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
                    def setattr(self_inner, target, name, value=_NOTSET):
                        if value is self_inner._NOTSET:
                            value = name
                            parts = target.rsplit('.', 1)
                            target = importlib.import_module(parts[0]) if len(parts) == 2 else target
                            name = parts[-1]
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
                    def undo(self_inner):
                        for target, name, old in reversed(self_inner._patches):
                            setattr(target, name, old)
                        self_inner._patches.clear()
                        for key, old in reversed(self_inner._env_patches):
                            if old is None: os.environ.pop(key, None)
                            else: os.environ[key] = old
                        self_inner._env_patches.clear()
                fixture_kwargs["monkeypatch"] = _MonkeyPatch()
                fixture_cleanups.append(lambda: fixture_kwargs["monkeypatch"].undo())
            else:
                # Try loading from conftest.py
                conftest_dir = test_dir
                while conftest_dir:
                    cp = os.path.join(conftest_dir, "conftest.py")
                    if os.path.exists(cp):
                        if "conftest" in sys.modules:
                            del sys.modules["conftest"]
                        import pytest as _pt
                        _orig_fix = _pt.fixture
                        _pt.fixture = lambda f=None, **kw: f if f is not None else (lambda fn: fn)
                        spec = importlib.util.spec_from_file_location("conftest", cp)
                        cmod = importlib.util.module_from_spec(spec)
                        spec.loader.exec_module(cmod)
                        _pt.fixture = _orig_fix
                        if hasattr(cmod, dep):
                            fixture_kwargs[dep] = getattr(cmod, dep)()
                        break
                    parent = os.path.dirname(conftest_dir)
                    if parent == conftest_dir:
                        break
                    conftest_dir = parent

        # Execute test
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
        if asyncio.iscoroutine(result):
            asyncio.run(result)
        # Test passed — check if xfail (passed when expected to fail = XPassed)
        if xfail_marker is not None:
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
        return {
            "test_id": test_item["id"],
            "outcome": "Failed",
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
