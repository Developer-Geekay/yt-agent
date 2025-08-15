use anyhow::{anyhow, Result};
use directories::{ProjectDirs, UserDirs};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

/// The structure of our configuration file (config.toml)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub download_directory: String,
}

impl Default for Config {
    fn default() -> Self {
        // Use the 'directories' crate to find the user's download directory.
        // This works on Windows, macOS, and Linux.
        let default_dir = UserDirs::new()
            .and_then(|dirs| dirs.download_dir().map(|p| p.to_string_lossy().to_string()))
            .unwrap_or_else(|| "downloads".to_string()); // Fallback

        Config {
            download_directory: default_dir,
        }
    }
}

// --- THIS IS THE CORRECTED FUNCTION ---
/// Returns the cross-platform path to the configuration file, creating the directory if needed.
async fn get_config_path() -> Result<PathBuf> {
    // This part is synchronous and can fail, so we handle it first.
    let project_dirs = ProjectDirs::from("com", "YourOrg", "YT-DLP-API")
        .ok_or_else(|| anyhow!("Could not find a valid home directory to store config"))?;

    let config_dir = project_dirs.config_dir();

    // This part is asynchronous and is now correctly awaited.
    fs::create_dir_all(config_dir).await?;

    Ok(config_dir.join("config.toml"))
}

/// Loads the configuration from the file, or creates a default one if it doesn't exist.
pub async fn load_config() -> Result<Config> {
    // The call to the async function is now correctly awaited.
    let config_path = get_config_path().await?;

    if !config_path.exists() {
        tracing::info!(
            "No config file found. Creating a default one at: {}",
            config_path.display()
        );
        let default_config = Config::default();
        save_config(&default_config).await?;
        return Ok(default_config);
    }

    let config_content = fs::read_to_string(&config_path).await?;
    let config: Config = toml::from_str(&config_content)
        .map_err(|e| anyhow!("Failed to parse config file at {}: {}", config_path.display(), e))?;

    Ok(config)
}

/// Saves the provided configuration object to the file.
pub async fn save_config(config: &Config) -> Result<()> {
    // The call to the async function is now correctly awaited.
    let config_path = get_config_path().await?;
    let toml_string = toml::to_string_pretty(config)?;
    fs::write(config_path, toml_string).await?;
    Ok(())
}
