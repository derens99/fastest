# Fastest installer for Windows
# Usage: irm https://raw.githubusercontent.com/derens99/fastest/main/install.ps1 | iex

param(
    [string]$Version = "latest",
    [string]$InstallDir = "$env:LOCALAPPDATA\fastest\bin"
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

# Configuration
$Repo = "derens99/fastest"
$BinaryName = "fastest.exe"

# Helper functions
function Write-Info {
    param([string]$Message)
    Write-Host "info: " -ForegroundColor Blue -NoNewline
    Write-Host $Message
}

function Write-Success {
    param([string]$Message)
    Write-Host "success: " -ForegroundColor Green -NoNewline
    Write-Host $Message
}

function Write-Warning {
    param([string]$Message)
    Write-Host "warning: " -ForegroundColor Yellow -NoNewline
    Write-Host $Message
}

function Write-Error {
    param([string]$Message)
    Write-Host "error: " -ForegroundColor Red -NoNewline
    Write-Host $Message
}

# Detect architecture
function Get-Architecture {
    $arch = $env:PROCESSOR_ARCHITECTURE
    switch ($arch) {
        "AMD64" { return "x86_64" }
        "ARM64" { return "aarch64" }
        default {
            Write-Error "Unsupported architecture: $arch"
            exit 1
        }
    }
}

# Get latest version from GitHub
function Get-LatestVersion {
    try {
        $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
        return $release.tag_name
    }
    catch {
        Write-Error "Failed to get latest version: $_"
        exit 1
    }
}

# Download and install
function Install-Fastest {
    param(
        [string]$Version,
        [string]$Architecture
    )
    
    # Get version if not specified
    if ($Version -eq "latest") {
        $Version = Get-LatestVersion
        Write-Info "Latest version: $Version"
    }
    
    # Construct download URL
    $platform = "$Architecture-pc-windows-msvc"
    $url = "https://github.com/$Repo/releases/download/$Version/fastest-$platform.zip"
    
    Write-Info "Downloading fastest $Version for Windows $Architecture..."
    
    # Create temp directory
    $tempDir = New-TemporaryFile | ForEach-Object { Remove-Item $_; New-Item -ItemType Directory -Path $_ }
    $zipPath = Join-Path $tempDir "fastest.zip"
    
    try {
        # Download
        Invoke-WebRequest -Uri $url -OutFile $zipPath -UseBasicParsing
        
        # Extract
        Write-Info "Extracting archive..."
        Expand-Archive -Path $zipPath -DestinationPath $tempDir -Force
        
        # Create install directory
        if (!(Test-Path $InstallDir)) {
            New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        }
        
        # Install binary
        $sourcePath = Join-Path $tempDir $BinaryName
        $destPath = Join-Path $InstallDir $BinaryName
        
        Write-Info "Installing to $destPath..."
        Move-Item -Path $sourcePath -Destination $destPath -Force
        
        # Verify installation
        if (Test-Path $destPath) {
            & $destPath --version | Out-Null
            Write-Success "fastest installed successfully!"
        }
        else {
            Write-Error "Installation failed"
            exit 1
        }
    }
    finally {
        # Cleanup
        Remove-Item -Path $tempDir -Recurse -Force
    }
}

# Add to PATH
function Add-ToPath {
    param([string]$Dir)
    
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    
    if ($userPath -notlike "*$Dir*") {
        Write-Info "Adding $Dir to user PATH..."
        
        $newPath = if ($userPath) { "$userPath;$Dir" } else { $Dir }
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        
        # Update current session
        $env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + $newPath
        
        Write-Warning "PATH updated. You may need to restart your terminal."
    }
    else {
        Write-Info "PATH already contains $Dir"
    }
}

# Main
function Main {
    Write-Host "🚀 Installing fastest - The blazing fast Python test runner" -ForegroundColor Cyan
    Write-Host ""
    
    # Get architecture
    $arch = Get-Architecture
    Write-Info "Detected architecture: $arch"
    
    # Install
    Install-Fastest -Version $Version -Architecture $arch
    
    # Add to PATH
    Add-ToPath -Dir $InstallDir
    
    Write-Host ""
    Write-Success "Installation complete! 🎉"
    Write-Host ""
    Write-Host "To get started, try:"
    Write-Host "  fastest --help"
    Write-Host "  fastest tests/"
    Write-Host ""
    Write-Host "For more information, visit: https://github.com/$Repo"
}

# Run main
Main 