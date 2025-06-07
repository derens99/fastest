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

#[derive(Debug, Serialize, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: Option<String>,
    published_at: Option<String>,
}

const VERSION_MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/derens99/fastest/main/.github/version.json";
const GITHUB_API_LATEST_RELEASE: &str = 
    "https://api.github.com/repos/derens99/fastest/releases/latest";

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
        // First try GitHub API for the latest release
        match self.fetch_latest_from_github() {
            Ok(latest_version) => {
                let version_without_v = latest_version.trim_start_matches('v');
                if self.is_newer_version(version_without_v, &self.current_version) {
                    Ok(Some(version_without_v.to_string()))
                } else {
                    Ok(None)
                }
            }
            Err(_) => {
                // Fallback to version manifest if GitHub API fails
                let manifest = self.fetch_version_manifest()?;
                if self.is_newer_version(&manifest.latest, &self.current_version) {
                    Ok(Some(manifest.latest))
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Perform the update
    pub fn update(&self, verbose: bool) -> Result<()> {
        // Try to get latest version from GitHub API first
        let latest_version = match self.fetch_latest_from_github() {
            Ok(tag) => tag.trim_start_matches('v').to_string(),
            Err(_) => {
                // Fallback to version manifest
                let manifest = self.fetch_version_manifest()?;
                manifest.latest
            }
        };

        if !self.is_newer_version(&latest_version, &self.current_version) {
            println!(
                "You are already running the latest version (v{})!",
                self.current_version
            );
            return Ok(());
        }

        println!("Current version: v{}", self.current_version);
        println!("Latest version: v{}", latest_version);
        println!("Updating to v{}...\n", latest_version);

        // Determine platform
        let platform = self.get_platform()?;
        if verbose {
            eprintln!("Detected platform: {}", platform);
        }

        // Construct download URLs based on platform
        let ext = if platform.contains("windows") { "zip" } else { "tar.gz" };
        let version_tag = format!("v{}", latest_version);
        
        let download_url = format!(
            "https://github.com/derens99/fastest/releases/download/{}/fastest-{}-{}.{}",
            version_tag, version_tag, platform, ext
        );
        
        let checksum_url = format!(
            "https://github.com/derens99/fastest/releases/download/{}/fastest-{}-{}.{}.sha256sum",
            version_tag, version_tag, platform, ext
        );

        if verbose {
            eprintln!("Download URL: {}", download_url);
            eprintln!("Checksum URL: {}", checksum_url);
        }

        // Create temporary directory
        let temp_dir = std::env::temp_dir().join("fastest-update");
        fs::create_dir_all(&temp_dir)?;

        // Download the binary
        println!("Downloading fastest v{}...", latest_version);
        let archive_path = self.download_file(&download_url, &temp_dir, verbose)?;

        // Download and verify checksum
        println!("Verifying checksum...");
        let checksum_path = self.download_file(&checksum_url, &temp_dir, verbose)?;
        self.verify_checksum(&archive_path, &checksum_path)?;

        // Extract the binary
        println!("Extracting binary...");
        let binary_path = self.extract_binary(&archive_path, &temp_dir, &platform)?;

        // Replace the current binary
        println!("Installing new version...");
        self.replace_binary(&binary_path)?;

        // Clean up
        fs::remove_dir_all(&temp_dir).ok();

        println!("\n✅ Successfully updated to fastest v{}!", latest_version);
        println!("Run 'fastest --version' to verify the update.");

        Ok(())
    }

    fn fetch_latest_from_github(&self) -> Result<String> {
        let response = ureq::get(GITHUB_API_LATEST_RELEASE)
            .set("User-Agent", "fastest-updater")
            .timeout(std::time::Duration::from_secs(10))
            .call()
            .context("Failed to fetch latest release from GitHub")?;

        let release: GitHubRelease = response
            .into_json()
            .context("Failed to parse GitHub release")?;

        Ok(release.tag_name)
    }

    fn fetch_version_manifest(&self) -> Result<VersionInfo> {
        let response = ureq::get(VERSION_MANIFEST_URL)
            .timeout(std::time::Duration::from_secs(10))
            .call()
            .context("Failed to fetch version manifest")?;

        let manifest: VersionInfo = response
            .into_json()
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
            ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
            ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
            ("macos", "x86_64") => "x86_64-apple-darwin",
            ("macos", "aarch64") => "aarch64-apple-darwin",
            ("windows", "x86_64") => "x86_64-pc-windows-msvc",
            _ => return Err(anyhow!("Unsupported platform: {}-{}", os, arch)),
        };

        Ok(platform.to_string())
    }

    fn download_file(&self, url: &str, temp_dir: &Path, verbose: bool) -> Result<PathBuf> {
        let filename = url
            .split('/')
            .last()
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
        use sha2::{Digest, Sha256};

        // Read expected checksum
        let checksum_content = fs::read_to_string(checksum_path)?;
        let expected_checksum = checksum_content
            .split_whitespace()
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
                expected_checksum,
                actual_checksum
            ));
        }

        Ok(())
    }

    fn extract_binary(
        &self,
        archive_path: &Path,
        temp_dir: &Path,
        platform: &str,
    ) -> Result<PathBuf> {
        let binary_name = if platform.contains("windows") {
            "fastest.exe"
        } else {
            "fastest"
        };
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
        fs::copy(new_binary, &current_exe).context(
            "Failed to replace binary. You may need to run with sudo/administrator privileges.",
        )?;

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
            eprintln!(
                "\n🎉 A new version of fastest is available: v{}",
                new_version
            );
            eprintln!(
                "   Run 'fastest update' to upgrade from v{} to v{}",
                env!("CARGO_PKG_VERSION"),
                new_version
            );
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
