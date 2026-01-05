use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{error, info, warn};
use tracing_subscriber::filter::LevelFilter;

/// OpenZT configuration file structure (openzt.toml)
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OpenZTConfig {
    #[serde(default)]
    pub mod_loading: ModLoadingConfig,

    #[serde(default)]
    pub logging: LoggingConfig,
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

/// Log level setting for OpenZT logging
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    /// Convert to tracing's LevelFilter
    pub fn to_level_filter(self) -> LevelFilter {
        match self {
            LogLevel::Trace => LevelFilter::TRACE,
            LogLevel::Debug => LevelFilter::DEBUG,
            LogLevel::Info => LevelFilter::INFO,
            LogLevel::Warn => LevelFilter::WARN,
            LogLevel::Error => LevelFilter::ERROR,
        }
    }
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Warn
    }
}

/// Logging configuration section
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LoggingConfig {
    /// Enable file logging to openzt.log (default: true)
    #[serde(default = "default_true")]
    pub log_to_file: bool,

    /// Log level (default: Warn)
    #[serde(default)]
    pub level: LogLevel,
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
            logging: LoggingConfig {
                log_to_file: true,
                level: LogLevel::Warn,
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

impl Default for LoggingConfig {
    fn default() -> Self {
        LoggingConfig {
            log_to_file: true,
            level: LogLevel::Warn,
        }
    }
}

/// Load OpenZT configuration from openzt.toml
///
/// Location: <Zoo Tycoon Install>/openzt.toml
///
/// If file doesn't exist, creates it with default values and returns default config.
/// If file exists but is missing sections, adds missing sections with defaults.
/// If file fails to parse, returns default config without overwriting the file.
pub fn load_openzt_config() -> OpenZTConfig {
    let config_path = get_config_path();

    if !config_path.exists() {
        info!("No openzt.toml found, creating with default configuration");
        let default_config = OpenZTConfig::default();

        // Save default config to file
        if let Err(e) = save_openzt_config(&default_config) {
            warn!("Failed to create openzt.toml: {}", e);
        }

        return default_config;
    }

    match std::fs::read_to_string(&config_path) {
        Ok(content) => {
            // Check which sections and fields are present in the file
            let needs_update = match toml::from_str::<toml::Value>(&content) {
                Ok(toml_value) => {
                    // Check if sections exist
                    let has_mod_loading = toml_value.get("mod_loading").is_some();
                    let has_logging = toml_value.get("logging").is_some();

                    // Check if all fields exist within sections
                    let mod_loading_complete = if let Some(mod_loading) = toml_value.get("mod_loading") {
                        mod_loading.get("order").is_some()
                            && mod_loading.get("disabled").is_some()
                            && mod_loading.get("auto_resolve_new_mods").is_some()
                            && mod_loading.get("warn_on_conflicts").is_some()
                    } else {
                        false
                    };

                    let logging_complete = if let Some(logging) = toml_value.get("logging") {
                        logging.get("log_to_file").is_some() && logging.get("level").is_some()
                    } else {
                        false
                    };

                    // Update needed if sections missing or fields incomplete
                    !has_mod_loading || !has_logging || !mod_loading_complete || !logging_complete
                }
                Err(_) => false, // If we can't parse as Value, the full parse will fail below
            };

            match toml::from_str::<OpenZTConfig>(&content) {
                Ok(config) => {
                    info!("Loaded OpenZT configuration from openzt.toml");

                    // If sections or fields were missing, save the complete config with defaults
                    if needs_update {
                        info!("Adding missing configuration sections/fields to openzt.toml");
                        if let Err(e) = save_openzt_config(&config) {
                            warn!("Failed to update openzt.toml with missing entries: {}", e);
                        }
                    }

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
         # This file controls mod loading order, logging, and other behavior\n\
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
        assert!(config.logging.log_to_file);
        assert_eq!(config.logging.level, LogLevel::Warn);
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
        assert_eq!(parsed.logging.log_to_file, config.logging.log_to_file);
        assert_eq!(parsed.logging.level, config.logging.level);
    }

    #[test]
    fn test_log_level_serialization() {
        // Test that log levels serialize to lowercase strings within a config
        let mut config = LoggingConfig::default();

        config.level = LogLevel::Trace;
        assert!(toml::to_string(&config).unwrap().contains("level = \"trace\""));

        config.level = LogLevel::Debug;
        assert!(toml::to_string(&config).unwrap().contains("level = \"debug\""));

        config.level = LogLevel::Info;
        assert!(toml::to_string(&config).unwrap().contains("level = \"info\""));

        config.level = LogLevel::Warn;
        assert!(toml::to_string(&config).unwrap().contains("level = \"warn\""));

        config.level = LogLevel::Error;
        assert!(toml::to_string(&config).unwrap().contains("level = \"error\""));
    }

    #[test]
    fn test_log_level_deserialization() {
        // Test that lowercase strings deserialize to LogLevel
        let trace_config: LoggingConfig = toml::from_str("enabled = true\nlevel = \"trace\"").unwrap();
        assert_eq!(trace_config.level, LogLevel::Trace);

        let debug_config: LoggingConfig = toml::from_str("enabled = true\nlevel = \"debug\"").unwrap();
        assert_eq!(debug_config.level, LogLevel::Debug);

        let info_config: LoggingConfig = toml::from_str("enabled = true\nlevel = \"info\"").unwrap();
        assert_eq!(info_config.level, LogLevel::Info);

        let warn_config: LoggingConfig = toml::from_str("enabled = true\nlevel = \"warn\"").unwrap();
        assert_eq!(warn_config.level, LogLevel::Warn);

        let error_config: LoggingConfig = toml::from_str("enabled = true\nlevel = \"error\"").unwrap();
        assert_eq!(error_config.level, LogLevel::Error);
    }

    #[test]
    fn test_log_level_to_level_filter() {
        // Test that LogLevel correctly converts to LevelFilter
        assert_eq!(LogLevel::Trace.to_level_filter(), LevelFilter::TRACE);
        assert_eq!(LogLevel::Debug.to_level_filter(), LevelFilter::DEBUG);
        assert_eq!(LogLevel::Info.to_level_filter(), LevelFilter::INFO);
        assert_eq!(LogLevel::Warn.to_level_filter(), LevelFilter::WARN);
        assert_eq!(LogLevel::Error.to_level_filter(), LevelFilter::ERROR);
    }

    #[test]
    fn test_logging_config_with_different_levels() {
        let config_str = "[logging]\nlog_to_file = false\nlevel = \"debug\"";

        let parsed: OpenZTConfig = toml::from_str(config_str).unwrap();
        assert!(!parsed.logging.log_to_file);
        assert_eq!(parsed.logging.level, LogLevel::Debug);
    }

    #[test]
    fn test_missing_logging_section_uses_defaults() {
        // Config file with only mod_loading section (older format)
        let config_str = r#"
[mod_loading]
order = ["mod_a", "mod_b"]
disabled = []
auto_resolve_new_mods = true
warn_on_conflicts = true
"#;

        let parsed: OpenZTConfig = toml::from_str(config_str).unwrap();

        // mod_loading should be loaded from file
        assert_eq!(parsed.mod_loading.order, vec!["mod_a", "mod_b"]);

        // logging should use defaults (serde's #[serde(default)])
        assert!(parsed.logging.log_to_file);
        assert_eq!(parsed.logging.level, LogLevel::Warn);
    }

    #[test]
    fn test_missing_mod_loading_section_uses_defaults() {
        // Config file with only logging section
        let config_str = r#"
[logging]
log_to_file = false
level = "error"
"#;

        let parsed: OpenZTConfig = toml::from_str(config_str).unwrap();

        // logging should be loaded from file
        assert!(!parsed.logging.log_to_file);
        assert_eq!(parsed.logging.level, LogLevel::Error);

        // mod_loading should use defaults
        assert!(parsed.mod_loading.order.is_empty());
        assert!(parsed.mod_loading.disabled.is_empty());
        assert!(parsed.mod_loading.auto_resolve_new_mods);
        assert!(parsed.mod_loading.warn_on_conflicts);
    }

    #[test]
    fn test_empty_config_uses_all_defaults() {
        // Empty config file
        let config_str = "";

        let parsed: OpenZTConfig = toml::from_str(config_str).unwrap();

        // Everything should use defaults
        assert!(parsed.mod_loading.order.is_empty());
        assert!(parsed.mod_loading.disabled.is_empty());
        assert!(parsed.mod_loading.auto_resolve_new_mods);
        assert!(parsed.mod_loading.warn_on_conflicts);
        assert!(parsed.logging.log_to_file);
        assert_eq!(parsed.logging.level, LogLevel::Warn);
    }

    #[test]
    fn test_missing_fields_within_logging_section() {
        // Logging section with only 'log_to_file' field (missing 'level')
        let config_str = r#"
[logging]
log_to_file = false
"#;

        let parsed: OpenZTConfig = toml::from_str(config_str).unwrap();

        // log_to_file should be from file
        assert!(!parsed.logging.log_to_file);

        // level should use default
        assert_eq!(parsed.logging.level, LogLevel::Warn);
    }

    #[test]
    fn test_missing_fields_within_mod_loading_section() {
        // mod_loading section with only 'order' field (missing other fields)
        let config_str = r#"
[mod_loading]
order = ["mod_a"]
"#;

        let parsed: OpenZTConfig = toml::from_str(config_str).unwrap();

        // order should be from file
        assert_eq!(parsed.mod_loading.order, vec!["mod_a"]);

        // Other fields should use defaults
        assert!(parsed.mod_loading.disabled.is_empty());
        assert!(parsed.mod_loading.auto_resolve_new_mods);
        assert!(parsed.mod_loading.warn_on_conflicts);
    }

    #[test]
    fn test_partial_fields_in_both_sections() {
        // Both sections present but each missing some fields
        let config_str = r#"
[mod_loading]
order = ["mod_a", "mod_b"]
warn_on_conflicts = false

[logging]
level = "debug"
"#;

        let parsed: OpenZTConfig = toml::from_str(config_str).unwrap();

        // mod_loading: order and warn_on_conflicts from file, rest defaults
        assert_eq!(parsed.mod_loading.order, vec!["mod_a", "mod_b"]);
        assert!(!parsed.mod_loading.warn_on_conflicts);
        assert!(parsed.mod_loading.disabled.is_empty()); // default
        assert!(parsed.mod_loading.auto_resolve_new_mods); // default

        // logging: level from file, log_to_file is default
        assert_eq!(parsed.logging.level, LogLevel::Debug);
        assert!(parsed.logging.log_to_file); // default
    }
}
