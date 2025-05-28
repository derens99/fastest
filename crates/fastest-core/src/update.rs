use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionInfo {
    pub latest: String,
    pub minimum: String,
    pub versions: std::collections::HashMap<String, ReleaseInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReleaseInfo {
    pub date: String,
    pub downloads: std::collections::HashMap<String, String>,
    pub checksums: std::collections::HashMap<String, String>,
}

const VERSION_MANIFEST_URL: &str = "https://raw.githubusercontent.com/derens99/fastest/main/.github/version.json";

pub struct UpdateChecker {
    current_version: String,
}

impl UpdateChecker {
    pub fn new() -> Self {
        Self {
            current_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Check if an update is available
    pub fn check_update(&self) -> Result<Option<String>> {
        let manifest = self.fetch_version_manifest()?;
        
        if self.is_newer_version(&manifest.latest, &self.current_version) {
            Ok(Some(manifest.latest))
        } else {
            Ok(None)
        }
    }

    /// Perform the update
    pub fn update(&self, verbose: bool) -> Result<()> {
        let manifest = self.fetch_version_manifest()?;
        
        if !self.is_newer_version(&manifest.latest, &self.current_version) {
            println!("You are already running the latest version (v{})!", self.current_version);
            return Ok(());
        }

        println!("Current version: v{}", self.current_version);
        println!("Latest version: v{}", manifest.latest);
        println!("Updating to v{}...\n", manifest.latest);

        // Determine platform
        let platform = self.get_platform()?;
        if verbose {
            eprintln!("Detected platform: {}", platform);
        }

        // Get download URL
        let release_info = manifest.versions.get(&manifest.latest)
            .ok_or_else(|| anyhow!("Release info not found for version {}", manifest.latest))?;
        
        let download_url = release_info.downloads.get(&platform)
            .ok_or_else(|| anyhow!("No download available for platform {}", platform))?;
        
        let checksum_url = release_info.checksums.get(&platform)
            .ok_or_else(|| anyhow!("No checksum available for platform {}", platform))?;

        if verbose {
            eprintln!("Download URL: {}", download_url);
            eprintln!("Checksum URL: {}", checksum_url);
        }

        // Create temporary directory
        let temp_dir = std::env::temp_dir().join("fastest-update");
        fs::create_dir_all(&temp_dir)?;

        // Download the binary
        println!("Downloading fastest v{}...", manifest.latest);
        let archive_path = self.download_file(download_url, &temp_dir, verbose)?;
        
        // Download and verify checksum
        println!("Verifying checksum...");
        let checksum_path = self.download_file(checksum_url, &temp_dir, verbose)?;
        self.verify_checksum(&archive_path, &checksum_path)?;

        // Extract the binary
        println!("Extracting binary...");
        let binary_path = self.extract_binary(&archive_path, &temp_dir, &platform)?;

        // Replace the current binary
        println!("Installing new version...");
        self.replace_binary(&binary_path)?;

        // Clean up
        fs::remove_dir_all(&temp_dir).ok();

        println!("\n✅ Successfully updated to fastest v{}!", manifest.latest);
        println!("Run 'fastest --version' to verify the update.");
        
        Ok(())
    }

    fn fetch_version_manifest(&self) -> Result<VersionInfo> {
        let response = ureq::get(VERSION_MANIFEST_URL)
            .timeout(std::time::Duration::from_secs(10))
            .call()
            .context("Failed to fetch version manifest")?;

        let manifest: VersionInfo = response.into_json()
            .context("Failed to parse version manifest")?;

        Ok(manifest)
    }

    fn is_newer_version(&self, new_version: &str, current_version: &str) -> bool {
        use semver::Version;
        
        match (Version::parse(new_version), Version::parse(current_version)) {
            (Ok(new), Ok(current)) => new > current,
            _ => false,
        }
    }

    fn get_platform(&self) -> Result<String> {
        let os = env::consts::OS;
        let arch = env::consts::ARCH;

        let platform = match (os, arch) {
            ("linux", "x86_64") => "linux-amd64",
            ("linux", "aarch64") => "linux-arm64",
            ("macos", "x86_64") => "darwin-amd64",
            ("macos", "aarch64") => "darwin-arm64",
            ("windows", "x86_64") => "windows-amd64",
            _ => return Err(anyhow!("Unsupported platform: {}-{}", os, arch)),
        };

        Ok(platform.to_string())
    }

    fn download_file(&self, url: &str, temp_dir: &Path, verbose: bool) -> Result<PathBuf> {
        let filename = url.split('/').last()
            .ok_or_else(|| anyhow!("Invalid URL: {}", url))?;
        let output_path = temp_dir.join(filename);

        if verbose {
            eprintln!("Downloading {} to {}", url, output_path.display());
        }

        let response = ureq::get(url)
            .timeout(std::time::Duration::from_secs(300))
            .call()
            .context("Failed to download file")?;

        let mut file = fs::File::create(&output_path)?;
        let mut reader = response.into_reader();
        std::io::copy(&mut reader, &mut file)?;

        Ok(output_path)
    }

    fn verify_checksum(&self, file_path: &Path, checksum_path: &Path) -> Result<()> {
        use sha2::{Sha256, Digest};
        
        // Read expected checksum
        let checksum_content = fs::read_to_string(checksum_path)?;
        let expected_checksum = checksum_content.split_whitespace()
            .next()
            .ok_or_else(|| anyhow!("Invalid checksum file format"))?
            .to_lowercase();

        // Calculate actual checksum
        let mut file = fs::File::open(file_path)?;
        let mut hasher = Sha256::new();
        std::io::copy(&mut file, &mut hasher)?;
        let actual_checksum = format!("{:x}", hasher.finalize());

        if expected_checksum != actual_checksum {
            return Err(anyhow!(
                "Checksum verification failed!\nExpected: {}\nActual: {}",
                expected_checksum, actual_checksum
            ));
        }

        Ok(())
    }

    fn extract_binary(&self, archive_path: &Path, temp_dir: &Path, platform: &str) -> Result<PathBuf> {
        let binary_name = if platform.contains("windows") { "fastest.exe" } else { "fastest" };
        let extracted_path = temp_dir.join(binary_name);

        if platform.contains("windows") {
            // Extract from zip
            use zip::ZipArchive;
            let file = fs::File::open(archive_path)?;
            let mut archive = ZipArchive::new(file)?;
            
            for i in 0..archive.len() {
                let mut file = archive.by_index(i)?;
                if file.name() == binary_name {
                    let mut output = fs::File::create(&extracted_path)?;
                    std::io::copy(&mut file, &mut output)?;
                    break;
                }
            }
        } else {
            // Extract from tar.gz
            use flate2::read::GzDecoder;
            use tar::Archive;
            
            let file = fs::File::open(archive_path)?;
            let gz = GzDecoder::new(file);
            let mut archive = Archive::new(gz);
            
            for entry in archive.entries()? {
                let mut entry = entry?;
                if entry.path()?.file_name() == Some(std::ffi::OsStr::new(binary_name)) {
                    entry.unpack(&extracted_path)?;
                    break;
                }
            }

            // Make executable on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&extracted_path)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&extracted_path, perms)?;
            }
        }

        if !extracted_path.exists() {
            return Err(anyhow!("Failed to extract binary from archive"));
        }

        Ok(extracted_path)
    }

    fn replace_binary(&self, new_binary: &Path) -> Result<()> {
        let current_exe = env::current_exe()?;
        
        // On Windows, we need to rename the old binary first
        #[cfg(windows)]
        {
            let backup_path = current_exe.with_extension("old");
            fs::rename(&current_exe, &backup_path).ok();
        }

        // Copy new binary to current location
        fs::copy(new_binary, &current_exe)
            .context("Failed to replace binary. You may need to run with sudo/administrator privileges.")?;

        // Clean up old binary on Windows
        #[cfg(windows)]
        {
            let backup_path = current_exe.with_extension("old");
            fs::remove_file(&backup_path).ok();
        }

        Ok(())
    }
}

/// Check for updates and print a message if available
pub fn check_for_updates() -> Result<()> {
    let checker = UpdateChecker::new();
    
    match checker.check_update() {
        Ok(Some(new_version)) => {
            eprintln!("\n🎉 A new version of fastest is available: v{}", new_version);
            eprintln!("   Run 'fastest update' to upgrade from v{} to v{}", 
                     env!("CARGO_PKG_VERSION"), new_version);
        }
        Ok(None) => {
            // No update available, don't print anything
        }
        Err(_) => {
            // Failed to check for updates, silently ignore
        }
    }
    
    Ok(())
}