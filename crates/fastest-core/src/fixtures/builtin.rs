/// Built-in fixture names
pub mod names {
    pub const TMP_PATH: &str = "tmp_path";
    pub const TMP_PATH_FACTORY: &str = "tmp_path_factory";
    pub const CAPSYS: &str = "capsys";
    pub const CAPFD: &str = "capfd";
    pub const MONKEYPATCH: &str = "monkeypatch";
    pub const REQUEST: &str = "request";
}

/// Generate Python code for built-in fixtures
pub fn generate_builtin_fixture_code(fixture_name: &str) -> Option<String> {
    match fixture_name {
        names::TMP_PATH => Some(generate_tmp_path_fixture()),
        names::TMP_PATH_FACTORY => Some(generate_tmp_path_fixture()), // Reuse for now
        names::CAPSYS => Some(generate_capsys_fixture()),
        names::MONKEYPATCH => Some(generate_monkeypatch_fixture()),
        _ => None,
    }
}

fn generate_tmp_path_fixture() -> String {
    r#"
import tempfile
import pathlib
import shutil
import weakref

class TmpPath:
    _cleanup_registry = []
    
    @classmethod
    def cleanup_all(cls):
        """Force cleanup of all temp directories"""
        for cleanup_func in cls._cleanup_registry:
            try:
                cleanup_func()
            except:
                pass
        cls._cleanup_registry.clear()
    
    def __init__(self):
        self.tmp_dir = tempfile.mkdtemp(prefix="fastest_")
        self.path = pathlib.Path(self.tmp_dir)
        
        # Use weakref to avoid circular references
        def cleanup():
            try:
                if self.tmp_dir and pathlib.Path(self.tmp_dir).exists():
                    shutil.rmtree(self.tmp_dir)
            except:
                pass
        
        # Register cleanup with weakref
        self._cleanup = cleanup
        TmpPath._cleanup_registry.append(cleanup)
        weakref.finalize(self, cleanup)
    
    def __str__(self):
        return str(self.path)
    
    def __fspath__(self):
        return str(self.path)
    
    def __truediv__(self, other):
        return self.path / other

def tmp_path_fixture():
    """Provide a temporary directory unique to the test invocation."""
    tmp = TmpPath()
    return tmp.path
"#
    .to_string()
}

fn generate_capsys_fixture() -> String {
    r#"
class SimpleCapsys:
    def __init__(self, stdout_buf, stderr_buf):
        self.stdout_buf = stdout_buf
        self.stderr_buf = stderr_buf
    
    def readouterr(self):
        out = self.stdout_buf.getvalue()
        err = self.stderr_buf.getvalue()
        self.stdout_buf.seek(0)
        self.stdout_buf.truncate()
        self.stderr_buf.seek(0)
        self.stderr_buf.truncate()
        
        # Return a named tuple-like object
        class CapturedOutput:
            def __init__(self, out, err):
                self.out = out
                self.err = err
        
        return CapturedOutput(out, err)

# This function will be called by the runner to create the capsys instance
# The runner will pass the current test's stdout_buf and stderr_buf
# def capsys_fixture(stdout_buf, stderr_buf):
#     return SimpleCapsys(stdout_buf, stderr_buf)

# For now, the runner will instantiate SimpleCapsys directly if 'capsys' is requested.
# The capsys_fixture() function isn't strictly needed if the runner handles instantiation.
"#
    .to_string()
}

fn generate_monkeypatch_fixture() -> String {
    r#"
class MonkeyPatch:
    def __init__(self):
        self._setattr = []
        self._setitem = []
        self._delattr = []
        self._delitem = []
    
    def setattr(self, obj, name, value):
        """Set attribute value, remembering the old value."""
        if hasattr(obj, name):
            old_value = getattr(obj, name)
            self._setattr.append((obj, name, old_value, True))
        else:
            self._setattr.append((obj, name, None, False))
        setattr(obj, name, value)
    
    def setenv(self, name, value):
        """Set environment variable."""
        import os
        self.setitem(os.environ, name, value)
    
    def delattr(self, obj, name):
        """Delete attribute."""
        if hasattr(obj, name):
            old_value = getattr(obj, name)
            self._delattr.append((obj, name, old_value))
            delattr(obj, name)
    
    def setitem(self, mapping, key, value):
        """Set item in mapping."""
        if key in mapping:
            old_value = mapping[key]
            self._setitem.append((mapping, key, old_value, True))
        else:
            self._setitem.append((mapping, key, None, False))
        mapping[key] = value
    
    def delitem(self, mapping, key):
        """Delete item from mapping."""
        if key in mapping:
            old_value = mapping[key]
            self._delitem.append((mapping, key, old_value))
            del mapping[key]
    
    def undo(self):
        """Undo all changes with proper error handling."""
        errors = []
        
        # Restore setattr changes
        for obj, name, value, existed in reversed(self._setattr):
            try:
                if existed:
                    setattr(obj, name, value)
                else:
                    delattr(obj, name)
            except Exception as e:
                errors.append(f"Failed to restore {obj}.{name}: {e}")
        
        # Restore delattr changes
        for obj, name, value in reversed(self._delattr):
            try:
                setattr(obj, name, value)
            except Exception as e:
                errors.append(f"Failed to restore deleted {obj}.{name}: {e}")
        
        # Restore setitem changes
        for mapping, key, value, existed in reversed(self._setitem):
            try:
                if existed:
                    mapping[key] = value
                else:
                    del mapping[key]
            except Exception as e:
                errors.append(f"Failed to restore mapping[{key}]: {e}")
        
        # Restore delitem changes
        for mapping, key, value in reversed(self._delitem):
            try:
                mapping[key] = value
            except Exception as e:
                errors.append(f"Failed to restore deleted mapping[{key}]: {e}")
        
        # Clear the tracking lists
        self._setattr.clear()
        self._delattr.clear()
        self._setitem.clear()
        self._delitem.clear()
        
        if errors:
            import warnings
            warnings.warn(f"MonkeyPatch undo errors: {'; '.join(errors)}")

def monkeypatch_fixture():
    """Monkeypatch fixture for modifying objects."""
    mp = MonkeyPatch()
    # Note: In real implementation, we'd register cleanup
    return mp
"#
    .to_string()
}

/// Check if a fixture is a built-in fixture
pub fn is_builtin_fixture(name: &str) -> bool {
    // Validate fixture name to prevent code injection
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return false;
    }
    
    matches!(
        name,
        names::TMP_PATH
            | names::TMP_PATH_FACTORY
            | names::CAPSYS
            | names::CAPFD
            | names::MONKEYPATCH
            | names::REQUEST
    )
}

/// Get fixture metadata for built-in fixtures
pub fn get_builtin_fixture_metadata(name: &str) -> Option<(String, String, bool)> {
    match name {
        names::TMP_PATH => Some(("function".to_string(), "tmp_path".to_string(), false)),
        names::CAPSYS => Some(("function".to_string(), "capsys".to_string(), false)),
        names::MONKEYPATCH => Some(("function".to_string(), "monkeypatch".to_string(), false)),
        _ => None,
    }
}
