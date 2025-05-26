# Fastest installer script for Windows
# Inspired by Astral's UV installer

param(
    [string]$InstallDir = "$env:USERPROFILE\.fastest",
    [switch]$NoModifyPath
)

$ErrorActionPreference = "Stop"

# Configuration
$REPO = "derens99/fastest"  # TODO: Update with actual GitHub username
$BASE_URL = "https://github.com/$REPO/releases"
$BIN_DIR = Join-Path $InstallDir "bin"
$EXECUTABLE_NAME = "fastest.exe"

# Helper functions
function Write-ColorOutput($ForegroundColor) {
    $fc = $host.UI.RawUI.ForegroundColor
    $host.UI.RawUI.ForegroundColor = $ForegroundColor
    if ($args) {
        Write-Output $args
    }
    $host.UI.RawUI.ForegroundColor = $fc
}

function Info($message) {
    Write-ColorOutput Blue "info: $message"
}

function Success($message) {
    Write-ColorOutput Green "success: $message"
}

function Warning($message) {
    Write-ColorOutput Yellow "warning: $message"
}

function ErrorMessage($message) {
    Write-ColorOutput Red "error: $message"
}

# Detect architecture
function Get-Architecture {
    $arch = [System.Environment]::GetEnvironmentVariable("PROCESSOR_ARCHITECTURE")
    switch ($arch) {
        "AMD64" { return "x86_64" }
        "ARM64" { return "aarch64" }
        default {
            ErrorMessage "Unsupported architecture: $arch"
            exit 1
        }
    }
}

# Download the latest release
function Download-Release {
    param($Architecture)
    
    $tempFile = [System.IO.Path]::GetTempFileName() + ".zip"
    $downloadUrl = "$BASE_URL/download/latest/fastest-windows-$Architecture.zip"
    
    Info "Finding latest release..."
    Info "Downloading Fastest for windows-$Architecture..."
    
    try {
        # Use TLS 1.2
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        
        $webClient = New-Object System.Net.WebClient
        $webClient.DownloadFile($downloadUrl, $tempFile)
    }
    catch {
        ErrorMessage "Failed to download Fastest: $_"
        exit 1
    }
    
    return $tempFile
}

# Install the binary
function Install-Binary {
    param($ArchiveFile)
    
    Info "Installing to $InstallDir..."
    
    # Create installation directory
    if (!(Test-Path $BIN_DIR)) {
        New-Item -ItemType Directory -Force -Path $BIN_DIR | Out-Null
    }
    
    # Extract the archive
    try {
        Add-Type -AssemblyName System.IO.Compression.FileSystem
        [System.IO.Compression.ZipFile]::ExtractToDirectory($ArchiveFile, $BIN_DIR)
    }
    catch {
        ErrorMessage "Failed to extract archive: $_"
        exit 1
    }
    
    # Clean up
    Remove-Item $ArchiveFile -Force
}

# Setup PATH
function Setup-Path {
    if ($NoModifyPath) {
        Warning "Not modifying PATH. Please add $BIN_DIR to your PATH manually."
        return
    }
    
    $User = [EnvironmentVariableTarget]::User
    $Path = [Environment]::GetEnvironmentVariable('Path', $User)
    
    if ($Path -notlike "*$BIN_DIR*") {
        Info "Adding $BIN_DIR to PATH..."
        
        # Add to user PATH
        $NewPath = "$BIN_DIR;$Path"
        [Environment]::SetEnvironmentVariable('Path', $NewPath, $User)
        
        # Update current session
        $env:Path = "$BIN_DIR;$env:Path"
        
        Info "PATH updated. You may need to restart your terminal."
    }
    else {
        Info "PATH already contains $BIN_DIR"
    }
}

# Verify installation
function Verify-Installation {
    $fastestPath = Join-Path $BIN_DIR $EXECUTABLE_NAME
    
    if (Test-Path $fastestPath) {
        try {
            $version = & $fastestPath --version 2>$null
            Success "Fastest $version installed successfully!"
        }
        catch {
            Warning "Installation completed but verification failed"
        }
    }
    else {
        ErrorMessage "Installation verification failed - executable not found"
        exit 1
    }
}

# Main installation flow
function Main {
    Write-Host "🚀 Installing Fastest - The blazing fast Python test runner" -ForegroundColor Cyan
    Write-Host ""
    
    # Check if already installed
    $fastestPath = Join-Path $BIN_DIR $EXECUTABLE_NAME
    if (Test-Path $fastestPath) {
        Warning "Fastest is already installed at $fastestPath"
        $response = Read-Host "Do you want to reinstall? [y/N]"
        if ($response -ne 'y' -and $response -ne 'Y') {
            exit 0
        }
    }
    
    # Detect architecture
    $arch = Get-Architecture
    Info "Detected architecture: windows-$arch"
    
    # Download release
    $tempArchive = Download-Release -Architecture $arch
    
    # Install binary
    Install-Binary -ArchiveFile $tempArchive
    
    # Setup PATH
    Setup-Path
    
    # Verify installation
    Verify-Installation
    
    Write-Host ""
    Write-Host "📚 Get started with:" -ForegroundColor Cyan
    Write-Host "    fastest --help"
    Write-Host ""
    Write-Host "📖 Documentation: https://github.com/$REPO" -ForegroundColor Blue
    Write-Host "🐛 Report issues: https://github.com/$REPO/issues" -ForegroundColor Blue
}

# Run main function
Main 