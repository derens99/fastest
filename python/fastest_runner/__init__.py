"""
Fastest Runner - A blazing fast Python test runner built with Rust

This package provides a Python wrapper around the fastest binary.
"""

import os
import sys
import platform
import subprocess
import urllib.request
import zipfile
import tarfile
import tempfile
import shutil
from pathlib import Path

__version__ = "0.1.0"
__all__ = ["main", "install_binary", "get_binary_path"]

REPO = "yourusername/fastest"
BINARY_NAME = "fastest.exe" if sys.platform == "win32" else "fastest"


def get_platform():
    """Detect the current platform."""
    system = platform.system().lower()
    machine = platform.machine().lower()
    
    # Map machine types
    if machine in ("x86_64", "amd64"):
        arch = "x86_64"
    elif machine in ("aarch64", "arm64"):
        arch = "aarch64"
    else:
        raise RuntimeError(f"Unsupported architecture: {machine}")
    
    # Map OS
    if system == "linux":
        os_name = "unknown-linux-gnu"
    elif system == "darwin":
        os_name = "apple-darwin"
    elif system == "windows":
        os_name = "pc-windows-msvc"
    else:
        raise RuntimeError(f"Unsupported OS: {system}")
    
    return f"{arch}-{os_name}"


def get_binary_path():
    """Get the path to the fastest binary."""
    # Check if binary is in package data
    package_dir = Path(__file__).parent
    binary_path = package_dir / "bin" / BINARY_NAME
    
    if binary_path.exists():
        return str(binary_path)
    
    # Check if binary is in PATH
    binary_in_path = shutil.which("fastest")
    if binary_in_path:
        return binary_in_path
    
    # Binary not found, need to install
    return None


def get_latest_version():
    """Get the latest release version from GitHub."""
    url = f"https://api.github.com/repos/{REPO}/releases/latest"
    
    with urllib.request.urlopen(url) as response:
        import json
        data = json.loads(response.read())
        return data["tag_name"]


def download_binary(version=None, progress=True):
    """Download the fastest binary for the current platform."""
    if version is None:
        version = get_latest_version()
    
    platform_str = get_platform()
    
    # Determine archive format
    if sys.platform == "win32":
        archive_ext = "zip"
    else:
        archive_ext = "tar.gz"
    
    url = f"https://github.com/{REPO}/releases/download/{version}/fastest-{platform_str}.{archive_ext}"
    
    # Download to temp file
    with tempfile.NamedTemporaryFile(delete=False, suffix=f".{archive_ext}") as tmp_file:
        if progress:
            print(f"Downloading fastest {version} for {platform_str}...")
        
        with urllib.request.urlopen(url) as response:
            shutil.copyfileobj(response, tmp_file)
        
        return tmp_file.name


def install_binary(version=None):
    """Install the fastest binary to the package directory."""
    package_dir = Path(__file__).parent
    bin_dir = package_dir / "bin"
    bin_dir.mkdir(exist_ok=True)
    
    # Download binary
    archive_path = download_binary(version)
    
    try:
        # Extract binary
        if archive_path.endswith(".zip"):
            with zipfile.ZipFile(archive_path, 'r') as zip_ref:
                # Extract just the binary
                for name in zip_ref.namelist():
                    if name.endswith(BINARY_NAME):
                        with zip_ref.open(name) as source, \
                             open(bin_dir / BINARY_NAME, 'wb') as target:
                            shutil.copyfileobj(source, target)
                        break
        else:
            with tarfile.open(archive_path, 'r:gz') as tar_ref:
                # Extract just the binary
                for member in tar_ref.getmembers():
                    if member.name.endswith(BINARY_NAME):
                        member.name = BINARY_NAME  # Flatten the path
                        tar_ref.extract(member, bin_dir)
                        break
        
        # Make executable on Unix
        if sys.platform != "win32":
            binary_path = bin_dir / BINARY_NAME
            os.chmod(binary_path, 0o755)
        
        print(f"Successfully installed fastest to {bin_dir}")
        
    finally:
        # Clean up
        os.unlink(archive_path)


def main():
    """Main entry point that wraps the fastest binary."""
    binary_path = get_binary_path()
    
    if not binary_path:
        print("fastest binary not found. Installing...")
        try:
            install_binary()
            binary_path = get_binary_path()
        except Exception as e:
            print(f"Failed to install fastest: {e}", file=sys.stderr)
            sys.exit(1)
    
    if not binary_path:
        print("Failed to locate fastest binary after installation", file=sys.stderr)
        sys.exit(1)
    
    # Run the binary with all arguments
    try:
        result = subprocess.run([binary_path] + sys.argv[1:])
        sys.exit(result.returncode)
    except KeyboardInterrupt:
        sys.exit(130)  # Standard exit code for Ctrl+C
    except Exception as e:
        print(f"Error running fastest: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main() 