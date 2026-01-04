use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{error, info, warn};

/// OpenZT configuration file structure (openzt.toml)
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OpenZTConfig {
    #[serde(default)]
    pub mod_loading: ModLoadingConfig,
}

/// Mod loading configuration section
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ModLoadingConfig {
    /// Explicit load order - mods are loaded in this sequence
    #[serde(default)]
    pub order: Vec<String>,

    /// Disabled mods (present in ./mods but should not load)
    #[serde(default)]
    pub disabled: Vec<String>,

    /// Auto-resolve new mods (default: true)
    #[serde(default = "default_true")]
    pub auto_resolve_new_mods: bool,

    /// Warn on conflicts (default: true)
    #[serde(default = "default_true")]
    pub warn_on_conflicts: bool,
}

fn default_true() -> bool {
    true
}

impl Default for OpenZTConfig {
    fn default() -> Self {
        OpenZTConfig {
            mod_loading: ModLoadingConfig {
                order: Vec::new(),
                disabled: Vec::new(),
                auto_resolve_new_mods: true,
                warn_on_conflicts: true,
            },
        }
    }
}

impl Default for ModLoadingConfig {
    fn default() -> Self {
        ModLoadingConfig {
            order: Vec::new(),
            disabled: Vec::new(),
            auto_resolve_new_mods: true,
            warn_on_conflicts: true,
        }
    }
}

/// Load OpenZT configuration from openzt.toml
///
/// Location: <Zoo Tycoon Install>/openzt.toml
///
/// If file doesn't exist or fails to parse, returns default config
pub fn load_openzt_config() -> OpenZTConfig {
    let config_path = get_config_path();

    if !config_path.exists() {
        info!("No openzt.toml found, using default configuration");
        return OpenZTConfig::default();
    }

    match std::fs::read_to_string(&config_path) {
        Ok(content) => {
            match toml::from_str::<OpenZTConfig>(&content) {
                Ok(config) => {
                    info!("Loaded OpenZT configuration from openzt.toml");
                    config
                }
                Err(e) => {
                    error!("Failed to parse openzt.toml: {}", e);
                    warn!("Using default configuration instead");
                    OpenZTConfig::default()
                }
            }
        }
        Err(e) => {
            warn!("Could not read openzt.toml: {}", e);
            warn!("Using default configuration");
            OpenZTConfig::default()
        }
    }
}

/// Save OpenZT configuration to openzt.toml
///
/// Uses atomic write (temp file + rename) to prevent corruption
pub fn save_openzt_config(config: &OpenZTConfig) -> anyhow::Result<()> {
    let config_path = get_config_path();
    let temp_path = get_temp_config_path();

    // Serialize with pretty formatting
    let toml_string = toml::to_string_pretty(config)
        .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))?;

    // Add header comment
    let content = format!(
        "# OpenZT Configuration File\n\
         # This file controls mod loading order and behavior\n\
         # Generated automatically - edit with caution\n\n{}",
        toml_string
    );

    // Write to temp file
    std::fs::write(&temp_path, content)
        .map_err(|e| anyhow::anyhow!("Failed to write temp config: {}", e))?;

    // Atomic rename (overwrites existing on Windows)
    std::fs::rename(&temp_path, &config_path)
        .map_err(|e| anyhow::anyhow!("Failed to rename temp config: {}", e))?;

    info!("Updated openzt.toml with new configuration");
    Ok(())
}

/// Get path to openzt.toml
fn get_config_path() -> PathBuf {
    crate::util::get_base_path().join("openzt.toml")
}

/// Get path to temporary config file for atomic writes
fn get_temp_config_path() -> PathBuf {
    crate::util::get_base_path().join("openzt.toml.tmp")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = OpenZTConfig::default();
        assert!(config.mod_loading.order.is_empty());
        assert!(config.mod_loading.disabled.is_empty());
        assert!(config.mod_loading.auto_resolve_new_mods);
        assert!(config.mod_loading.warn_on_conflicts);
    }

    #[test]
    fn test_serialize_deserialize() {
        let mut config = OpenZTConfig::default();
        config.mod_loading.order = vec![
            "mod_a".to_string(),
            "mod_b".to_string(),
            "mod_c".to_string(),
        ];
        config.mod_loading.disabled = vec!["disabled_mod".to_string()];

        let toml_str = toml::to_string(&config).unwrap();
        let parsed: OpenZTConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.mod_loading.order, config.mod_loading.order);
        assert_eq!(parsed.mod_loading.disabled, config.mod_loading.disabled);
        assert_eq!(parsed.mod_loading.auto_resolve_new_mods, config.mod_loading.auto_resolve_new_mods);
    }
}
