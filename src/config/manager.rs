//! Configuration manager
//!
//! Handles loading, saving, and managing application configuration.

use super::config::GestureConfig;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn, error};

const CONFIG_FILENAME: &str = "config.json";
const BACKUP_FILENAME: &str = "config.json.backup";

/// Configuration manager
pub struct ConfigManager {
    config_path: PathBuf,
    backup_path: PathBuf,
    config: GestureConfig,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new(config_dir: Option<PathBuf>) -> Result<Self> {
        let config_dir = config_dir
            .unwrap_or_else(|| {
                // Default to %APPDATA%\RustGesture
                let appdata = std::env::var("APPDATA")
                    .unwrap_or_else(|_| ".".to_string());
                PathBuf::from(appdata).join("RustGesture")
            });

        // Ensure config directory exists
        fs::create_dir_all(&config_dir)
            .context("Failed to create config directory")?;

        let config_path = config_dir.join(CONFIG_FILENAME);
        let backup_path = config_dir.join(BACKUP_FILENAME);

        let config = if config_path.exists() {
            Self::load_config(&config_path)?
        } else {
            info!("No config file found, creating default configuration");
            let default_config = GestureConfig::default();
            Self::save_config(&default_config, &config_path, &backup_path)?;
            default_config
        };

        Ok(Self {
            config_path,
            backup_path,
            config,
        })
    }

    /// Load configuration from file
    fn load_config(path: &Path) -> Result<GestureConfig> {
        info!("Loading configuration from: {:?}", path);

        let content = fs::read_to_string(path)
            .context("Failed to read config file")?;

        let config: GestureConfig = serde_json::from_str(&content)
            .context("Failed to parse config JSON")?;

        // Validate configuration version
        if config.version != 1 {
            warn!("Unknown config version: {}, attempting to load anyway", config.version);
        }

        info!("Configuration loaded successfully");
        Ok(config)
    }

    /// Save configuration to file
    fn save_config(config: &GestureConfig, config_path: &Path, backup_path: &Path) -> Result<()> {
        info!("Saving configuration to: {:?}", config_path);

        // Create backup if old config exists
        if config_path.exists() {
            if let Err(e) = fs::copy(config_path, backup_path) {
                warn!("Failed to create backup: {}", e);
            }
        }

        // Serialize config to JSON
        let json = serde_json::to_string_pretty(config)
            .context("Failed to serialize config")?;

        // Write to temp file first (atomic write)
        let temp_path = config_path.with_extension("tmp");
        fs::write(&temp_path, json)
            .context("Failed to write temp config file")?;

        // Rename temp file to actual config file
        fs::rename(&temp_path, config_path)
            .context("Failed to rename temp config file")?;

        info!("Configuration saved successfully");
        Ok(())
    }

    /// Get the current configuration
    pub fn config(&self) -> &GestureConfig {
        &self.config
    }

    /// Get mutable reference to configuration
    pub fn config_mut(&mut self) -> &mut GestureConfig {
        &mut self.config
    }

    /// Save the current configuration
    pub fn save(&self) -> Result<()> {
        Self::save_config(&self.config, &self.config_path, &self.backup_path)
    }

    /// Reload configuration from file
    pub fn reload(&mut self) -> Result<()> {
        info!("Reloading configuration");
        self.config = Self::load_config(&self.config_path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_manager_create() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = Some(temp_dir.path().to_path_buf());

        let manager = ConfigManager::new(config_dir).unwrap();
        assert!(!manager.config().global_gestures.is_empty());
    }

    #[test]
    fn test_config_save_and_reload() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = Some(temp_dir.path().to_path_buf());

        let mut manager = ConfigManager::new(config_dir).unwrap();
        manager.save().unwrap();

        let config_path = manager.config_path.clone();
        let mut manager2 = ConfigManager::new(Some(config_path.parent().unwrap().to_path_buf())).unwrap();
        manager2.reload().unwrap();

        // Configs should be equal
        assert_eq!(manager.config().version, manager2.config().version);
    }
}
