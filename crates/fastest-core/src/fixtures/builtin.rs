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
import atexit

def tmp_path():
    """Provide a temporary directory unique to the test invocation."""
    tmp_dir = tempfile.mkdtemp(prefix="fastest_")
    path = pathlib.Path(tmp_dir)
    
    # Register cleanup
    def cleanup():
        try:
            shutil.rmtree(tmp_dir)
        except:
            pass
    atexit.register(cleanup)
    
    return path
"#.to_string()
}

fn generate_capsys_fixture() -> String {
    r#"
import sys
import io

class CaptureFixture:
    def __init__(self):
        self._stdout = None
        self._stderr = None
        self._stdout_backup = None
        self._stderr_backup = None
    
    def readouterr(self):
        """Read and return captured output."""
        out = self._stdout.getvalue() if self._stdout else ""
        err = self._stderr.getvalue() if self._stderr else ""
        if self._stdout:
            self._stdout.seek(0)
            self._stdout.truncate()
        if self._stderr:
            self._stderr.seek(0)
            self._stderr.truncate()
        return (out, err)
    
    def __enter__(self):
        self._stdout_backup = sys.stdout
        self._stderr_backup = sys.stderr
        self._stdout = io.StringIO()
        self._stderr = io.StringIO()
        sys.stdout = self._stdout
        sys.stderr = self._stderr
        return self
    
    def __exit__(self, *args):
        sys.stdout = self._stdout_backup
        sys.stderr = self._stderr_backup

def capsys():
    """Capture stdout/stderr output."""
    return CaptureFixture()
"#.to_string()
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
        """Undo all changes."""
        # Restore setattr changes
        for obj, name, value, existed in reversed(self._setattr):
            if existed:
                setattr(obj, name, value)
            else:
                delattr(obj, name)
        
        # Restore delattr changes
        for obj, name, value in reversed(self._delattr):
            setattr(obj, name, value)
        
        # Restore setitem changes
        for mapping, key, value, existed in reversed(self._setitem):
            if existed:
                mapping[key] = value
            else:
                del mapping[key]
        
        # Restore delitem changes
        for mapping, key, value in reversed(self._delitem):
            mapping[key] = value

def monkeypatch():
    """Monkeypatch fixture for modifying objects."""
    mp = MonkeyPatch()
    # Note: In real implementation, we'd register cleanup
    return mp
"#.to_string()
}

/// Check if a fixture is a built-in fixture
pub fn is_builtin_fixture(name: &str) -> bool {
    matches!(
        name,
        names::TMP_PATH | names::TMP_PATH_FACTORY | names::CAPSYS | 
        names::CAPFD | names::MONKEYPATCH | names::REQUEST
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