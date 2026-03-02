import json, sys, time, traceback, importlib, io, os


def run_test(test_item):
    """Execute a single test item and return the result as a dict."""
    start = time.time()
    stdout_capture = io.StringIO()
    stderr_capture = io.StringIO()
    old_stdout, old_stderr = sys.stdout, sys.stderr
    try:
        sys.stdout, sys.stderr = stdout_capture, stderr_capture
        # Add test dir to path
        test_dir = os.path.dirname(os.path.abspath(test_item["path"]))
        if test_dir not in sys.path:
            sys.path.insert(0, test_dir)
        # Import module
        module_name = os.path.basename(test_item["path"]).replace(".py", "")
        mod = importlib.import_module(module_name)
        importlib.reload(mod)  # Ensure fresh import
        # Get and call test function
        if test_item.get("class_name"):
            cls = getattr(mod, test_item["class_name"])
            func = getattr(cls(), test_item["function_name"])
        else:
            func = getattr(mod, test_item["function_name"])
        func()
        return {
            "test_id": test_item["id"],
            "outcome": "Passed",
            "duration_ms": int((time.time() - start) * 1000),
            "error": None,
            "stdout": stdout_capture.getvalue(),
            "stderr": stderr_capture.getvalue(),
        }
    except Exception as e:
        return {
            "test_id": test_item["id"],
            "outcome": "Failed",
            "duration_ms": int((time.time() - start) * 1000),
            "error": traceback.format_exc(),
            "stdout": stdout_capture.getvalue(),
            "stderr": stderr_capture.getvalue(),
        }
    finally:
        sys.stdout, sys.stderr = old_stdout, old_stderr


for line in sys.stdin:
    line = line.strip()
    if not line or line == "EXIT":
        break
    try:
        result = run_test(json.loads(line))
        print(json.dumps(result), flush=True)
    except Exception as e:
        print(json.dumps({"error": str(e)}), flush=True)
