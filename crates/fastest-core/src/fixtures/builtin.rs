//! Built-in pytest fixture definitions and code generation.
//!
//! Provides the list of fixtures that pytest makes available out of the box,
//! and generates the Python setup code needed to emulate each one when
//! running tests outside of the full pytest harness.

/// Names of all built-in pytest fixtures recognised by Fastest.
pub const BUILTIN_FIXTURES: &[&str] = &[
    "tmp_path",
    "tmp_path_factory",
    "capsys",
    "capfd",
    "monkeypatch",
    "request",
    "pytestconfig",
    "cache",
];

/// Generate Python setup code that creates the value for a built-in fixture.
///
/// Returns `Some(code)` with a Python snippet that, when executed, binds a
/// local variable with the fixture name to an appropriate value.  Returns
/// `None` if the name is not a recognised built-in fixture.
pub fn generate_builtin_code(name: &str) -> Option<String> {
    let code = match name {
        "tmp_path" => "import tempfile, pathlib\n\
             tmp_path = pathlib.Path(tempfile.mkdtemp())"
            .to_string(),
        "tmp_path_factory" => "import tempfile, pathlib\n\
             class _TmpPathFactory:\n\
             \x20   def __init__(self):\n\
             \x20       self._base = pathlib.Path(tempfile.mkdtemp())\n\
             \x20       self._counter = 0\n\
             \x20   def mktemp(self, basename='test'):\n\
             \x20       self._counter += 1\n\
             \x20       p = self._base / f'{basename}_{self._counter}'\n\
             \x20       p.mkdir(parents=True, exist_ok=True)\n\
             \x20       return p\n\
             tmp_path_factory = _TmpPathFactory()"
            .to_string(),
        "capsys" => "import io, sys\n\
             class _CapturedOutput:\n\
             \x20   def __init__(self):\n\
             \x20       self.out = ''\n\
             \x20       self.err = ''\n\
             class _Capsys:\n\
             \x20   def __init__(self):\n\
             \x20       self._old_stdout = sys.stdout\n\
             \x20       self._old_stderr = sys.stderr\n\
             \x20       sys.stdout = io.StringIO()\n\
             \x20       sys.stderr = io.StringIO()\n\
             \x20   def readouterr(self):\n\
             \x20       out = sys.stdout.getvalue() if hasattr(sys.stdout, 'getvalue') else ''\n\
             \x20       err = sys.stderr.getvalue() if hasattr(sys.stderr, 'getvalue') else ''\n\
             \x20       sys.stdout = io.StringIO()\n\
             \x20       sys.stderr = io.StringIO()\n\
             \x20       result = _CapturedOutput()\n\
             \x20       result.out = out\n\
             \x20       result.err = err\n\
             \x20       return result\n\
             \x20   def _restore(self):\n\
             \x20       sys.stdout = self._old_stdout\n\
             \x20       sys.stderr = self._old_stderr\n\
             capsys = _Capsys()\n\
             import atexit as _atexit\n\
             _atexit.register(capsys._restore)"
            .to_string(),
        "capfd" => "import io, sys\n\
             class _CapturedFd:\n\
             \x20   def __init__(self):\n\
             \x20       self.out = ''\n\
             \x20       self.err = ''\n\
             class _Capfd:\n\
             \x20   def readouterr(self):\n\
             \x20       result = _CapturedFd()\n\
             \x20       return result\n\
             capfd = _Capfd()"
            .to_string(),
        "monkeypatch" => "class _MonkeyPatch:\n\
             \x20   def __init__(self):\n\
             \x20       self._patches = []\n\
             \x20       self._env_patches = []\n\
             \x20   _NOTSET = object()\n\
             \x20   def setattr(self, target, name, value=_NOTSET):\n\
             \x20       if value is self._NOTSET:\n\
             \x20           # Two-arg form: setattr(\"pkg.mod.Class.attr\", value)\n\
             \x20           value = name\n\
             \x20           parts = target.rsplit('.', 1)\n\
             \x20           import importlib\n\
             \x20           target = importlib.import_module(parts[0]) if len(parts) == 2 else target\n\
             \x20           name = parts[-1]\n\
             \x20       old = getattr(target, name)\n\
             \x20       self._patches.append((target, name, old))\n\
             \x20       setattr(target, name, value)\n\
             \x20   def delattr(self, target, name):\n\
             \x20       old = getattr(target, name)\n\
             \x20       self._patches.append((target, name, old))\n\
             \x20       delattr(target, name)\n\
             \x20   def setenv(self, key, value):\n\
             \x20       import os\n\
             \x20       old = os.environ.get(key)\n\
             \x20       self._env_patches.append((key, old))\n\
             \x20       os.environ[key] = value\n\
             \x20   def delenv(self, key, raising=True):\n\
             \x20       import os\n\
             \x20       old = os.environ.get(key)\n\
             \x20       self._env_patches.append((key, old))\n\
             \x20       if key in os.environ:\n\
             \x20           del os.environ[key]\n\
             \x20       elif raising:\n\
             \x20           raise KeyError(key)\n\
             \x20   def undo(self):\n\
             \x20       import os\n\
             \x20       for target, name, old in reversed(self._patches):\n\
             \x20           setattr(target, name, old)\n\
             \x20       self._patches.clear()\n\
             \x20       for key, old in reversed(self._env_patches):\n\
             \x20           if old is None:\n\
             \x20               os.environ.pop(key, None)\n\
             \x20           else:\n\
             \x20               os.environ[key] = old\n\
             \x20       self._env_patches.clear()\n\
             monkeypatch = _MonkeyPatch()"
            .to_string(),
        "request" => "class _FixtureRequest:\n\
             \x20   def __init__(self):\n\
             \x20       self.param = None\n\
             \x20       self.node = None\n\
             \x20       self.config = None\n\
             \x20       self.fspath = None\n\
             request = _FixtureRequest()"
            .to_string(),
        "pytestconfig" => "class _PytestConfig:\n\
             \x20   def __init__(self):\n\
             \x20       self.rootdir = '.'\n\
             \x20       self.inipath = None\n\
             \x20   def getini(self, name):\n\
             \x20       return None\n\
             \x20   def getoption(self, name, default=None):\n\
             \x20       return default\n\
             pytestconfig = _PytestConfig()"
            .to_string(),
        "cache" => "class _Cache:\n\
             \x20   def __init__(self):\n\
             \x20       self._data = {}\n\
             \x20   def get(self, key, default):\n\
             \x20       return self._data.get(key, default)\n\
             \x20   def set(self, key, value):\n\
             \x20       self._data[key] = value\n\
             cache = _Cache()"
            .to_string(),
        _ => return None,
    };
    Some(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_builtin_code() {
        // Every recognised built-in should produce some code
        for name in BUILTIN_FIXTURES {
            let code = generate_builtin_code(name);
            assert!(
                code.is_some(),
                "generate_builtin_code returned None for built-in '{}'",
                name,
            );
            let code = code.unwrap();
            assert!(
                !code.is_empty(),
                "generate_builtin_code returned empty code for built-in '{}'",
                name,
            );
            // The generated code should define a variable with the fixture name
            assert!(
                code.contains(name),
                "generated code for '{}' does not contain the fixture name",
                name,
            );
        }
    }

    #[test]
    fn test_all_builtins_recognized() {
        use crate::fixtures::is_builtin;

        for name in BUILTIN_FIXTURES {
            assert!(
                is_builtin(name),
                "'{}' should be recognised as a built-in fixture",
                name,
            );
        }

        // Non-builtins should not be recognised
        assert!(!is_builtin("my_db"));
        assert!(!is_builtin("setup_env"));
        assert!(!is_builtin(""));
    }

    #[test]
    fn test_unknown_fixture_returns_none() {
        assert!(generate_builtin_code("not_a_builtin").is_none());
        assert!(generate_builtin_code("").is_none());
    }

    #[test]
    fn test_tmp_path_code_creates_pathlib_path() {
        let code = generate_builtin_code("tmp_path").unwrap();
        assert!(code.contains("pathlib.Path"));
        assert!(code.contains("tempfile.mkdtemp"));
    }

    #[test]
    fn test_capsys_code_captures_stdout() {
        let code = generate_builtin_code("capsys").unwrap();
        assert!(code.contains("sys.stdout"));
        assert!(code.contains("readouterr"));
    }

    #[test]
    fn test_monkeypatch_code_has_setattr() {
        let code = generate_builtin_code("monkeypatch").unwrap();
        assert!(code.contains("setattr"));
        assert!(code.contains("setenv"));
        assert!(code.contains("delenv"));
        assert!(code.contains("undo"));
    }
}
