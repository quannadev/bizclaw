//! OTA Updater for BizClaw.
//! Automatically downloads new binaries from GitHub Releases
//! and replaces the current executable using `self_replace`.

use reqwest::header::USER_AGENT;
use serde::Deserialize;
use std::env;

use thiserror::Error;
use tracing::{error, info, warn};

#[derive(Error, Debug)]
pub enum UpdaterError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Self replace error: {0}")]
    ReplaceError(String),
    #[error("No update available")]
    NoUpdateAvailable,
    #[error("Parse error")]
    ParseError,
}

#[derive(Deserialize, Debug)]
pub struct Release {
    pub tag_name: String,
    pub assets: Vec<Asset>,
}

#[derive(Deserialize, Debug)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}

pub struct OtaUpdater {
    repo: String,            // e.g., "nguyenduchoai/bizclaw-cloud"
    current_version: String, // e.g., "v1.0.10"
}

impl OtaUpdater {
    pub fn new(repo: &str, current_version: &str) -> Self {
        Self {
            repo: repo.to_string(),
            current_version: current_version.to_string(),
        }
    }

    /// Checks for a new release on GitHub and executes self-replacement
    pub async fn check_and_apply_update(&self) -> Result<bool, UpdaterError> {
        info!(
            "Checking for updates on {} (Current: {})",
            self.repo, self.current_version
        );

        let client = reqwest::Client::new();
        let release_url = format!("https://api.github.com/repos/{}/releases/latest", self.repo);

        let release_resp = client
            .get(&release_url)
            .header(USER_AGENT, "BizClaw-Updater")
            .send()
            .await?;

        if !release_resp.status().is_success() {
            warn!("Failed to fetch latest release: {}", release_resp.status());
            return Err(UpdaterError::ParseError);
        }

        let release: Release = release_resp.json().await?;

        // Compare version strings trivially (assumes vX.Y.Z format)
        if release.tag_name <= self.current_version {
            info!("BizClaw is up to date.");
            return Ok(false);
        }

        info!("New version available: {}", release.tag_name);

        // Find the right asset for the current platform
        let target_os = env::consts::OS;
        let target_arch = env::consts::ARCH;

        // E.g. "bizclaw-linux-x86_64" or "bizclaw-windows-x86_64.exe"
        let asset_identifier = format!("{}-{}", target_os, target_arch);

        // Discard installers (msi, deb, dmg). We just want the raw binary to hot-replace.
        let target_asset = release.assets.iter().find(|a| {
            let name_lower = a.name.to_lowercase();
            name_lower.contains(&asset_identifier)
                && !name_lower.ends_with(".deb")
                && !name_lower.ends_with(".msi")
                && !name_lower.ends_with(".dmg")
                && !name_lower.ends_with(".zip")
                && !name_lower.ends_with(".tar.gz")
        });

        if let Some(asset) = target_asset {
            info!("Downloading binary from: {}", asset.browser_download_url);
            let response = client
                .get(&asset.browser_download_url)
                .header(USER_AGENT, "BizClaw-Updater")
                .send()
                .await?;

            let bytes = response.bytes().await?;

            // Write to a temporary file
            info!("Writing downloaded binary to temp file...");
            let mut temp_file = tempfile::NamedTempFile::new()?;
            std::io::copy(&mut bytes.as_ref(), &mut temp_file)?;

            // Apply the self replacement
            info!("Applying update via self_replace...");
            if let Err(e) = self_replace::self_replace(temp_file.path()) {
                error!("Failed to self_replace: {}", e);
                return Err(UpdaterError::ReplaceError(e.to_string()));
            }

            // Cleanup temp file
            if let Err(e) = std::fs::remove_file(temp_file.path()) {
                warn!("Warning: Failed to cleanup temp file: {}", e);
            }

            info!("✨ OTA Update applied successfully! Restart required to take effect.");
            Ok(true)
        } else {
            warn!(
                "No compatible binary found for platform: {}",
                asset_identifier
            );
            Err(UpdaterError::ParseError)
        }
    }
}
