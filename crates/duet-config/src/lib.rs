use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("Migration error: {0}")]
    MigrationFailed(String),

    #[error("Watcher error: {0}")]
    WatchFailed(String),
}

pub type ConfigResult<T> = Result<T, ConfigError>;

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Application settings configuration (`settings.toml`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SettingsConfig {
    #[serde(default = "default_schema_version")]
    pub version: u32,

    #[serde(default = "default_true")]
    pub directories_first: bool,

    #[serde(default = "default_false")]
    pub show_hidden: bool,

    #[serde(default = "default_true")]
    pub confirm_delete: bool,

    #[serde(default = "default_sort_column")]
    pub default_sort_column: String,

    #[serde(default = "default_true")]
    pub default_sort_ascending: bool,

    #[serde(flatten)]
    pub extra_fields: BTreeMap<String, toml::Value>,
}

fn default_schema_version() -> u32 {
    CURRENT_SCHEMA_VERSION
}
fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}
fn default_sort_column() -> String {
    "name".to_string()
}

impl Default for SettingsConfig {
    fn default() -> Self {
        Self {
            version: CURRENT_SCHEMA_VERSION,
            directories_first: true,
            show_hidden: false,
            confirm_delete: true,
            default_sort_column: default_sort_column(),
            default_sort_ascending: true,
            extra_fields: BTreeMap::new(),
        }
    }
}

/// Keyboard shortcuts configuration (`keymap.toml`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeymapConfig {
    #[serde(default = "default_schema_version")]
    pub version: u32,

    #[serde(default)]
    pub bindings: HashMap<String, String>,

    #[serde(flatten)]
    pub extra_fields: BTreeMap<String, toml::Value>,
}

impl Default for KeymapConfig {
    fn default() -> Self {
        Self {
            version: CURRENT_SCHEMA_VERSION,
            bindings: HashMap::new(),
            extra_fields: BTreeMap::new(),
        }
    }
}

/// Visual theme configuration (`theme.toml`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThemeConfig {
    #[serde(default = "default_schema_version")]
    pub version: u32,

    #[serde(default = "default_theme_name")]
    pub name: String,

    #[serde(default)]
    pub colors: HashMap<String, String>,

    #[serde(flatten)]
    pub extra_fields: BTreeMap<String, toml::Value>,
}

fn default_theme_name() -> String {
    "default-dark".to_string()
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            version: CURRENT_SCHEMA_VERSION,
            name: default_theme_name(),
            colors: HashMap::new(),
            extra_fields: BTreeMap::new(),
        }
    }
}

/// Configuration manager handling TOML loading, schema versioning, migration runners, and hot reload watching.
pub struct ConfigLoader;

impl ConfigLoader {
    /// Load `settings.toml` with migration runner and unknown key preservation.
    pub fn load_settings(dir: &Path) -> ConfigResult<SettingsConfig> {
        let path = dir.join("settings.toml");
        if !path.exists() {
            let defaults = SettingsConfig::default();
            Self::save_to_file(&path, &defaults)?;
            return Ok(defaults);
        }

        let content = fs::read_to_string(&path)?;
        let raw_table: toml::Table = toml::from_str(&content)?;

        let version = raw_table
            .get("version")
            .and_then(|v| v.as_integer())
            .unwrap_or(0) as u32;

        if version < CURRENT_SCHEMA_VERSION {
            // Backup old version
            let backup_path = dir.join(format!("settings.toml.v{version}.bak"));
            fs::write(&backup_path, &content)?;

            // Migrate schema
            let mut settings: SettingsConfig = toml::from_str(&content).unwrap_or_default();
            settings.version = CURRENT_SCHEMA_VERSION;

            Self::save_to_file(&path, &settings)?;
            return Ok(settings);
        }

        let settings: SettingsConfig = toml::from_str(&content)?;
        Ok(settings)
    }

    /// Load `keymap.toml` with migration runner.
    pub fn load_keymap(dir: &Path) -> ConfigResult<KeymapConfig> {
        let path = dir.join("keymap.toml");
        if !path.exists() {
            let defaults = KeymapConfig::default();
            Self::save_to_file(&path, &defaults)?;
            return Ok(defaults);
        }

        let content = fs::read_to_string(&path)?;
        let keymap: KeymapConfig = toml::from_str(&content)?;
        Ok(keymap)
    }

    /// Load `theme.toml` with migration runner.
    pub fn load_theme(dir: &Path) -> ConfigResult<ThemeConfig> {
        let path = dir.join("theme.toml");
        if !path.exists() {
            let defaults = ThemeConfig::default();
            Self::save_to_file(&path, &defaults)?;
            return Ok(defaults);
        }

        let content = fs::read_to_string(&path)?;
        let theme: ThemeConfig = toml::from_str(&content)?;
        Ok(theme)
    }

    /// Save any serializable configuration to a TOML file.
    pub fn save_to_file<T: Serialize>(path: &Path, config: &T) -> ConfigResult<()> {
        let content = toml::to_string_pretty(config)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }
}

/// Hot reload configuration watcher monitoring configuration file changes.
pub struct ConfigWatcher {
    _watcher: notify::RecommendedWatcher,
    rx: Receiver<notify::Result<notify::Event>>,
}

impl ConfigWatcher {
    pub fn new(dir: &Path) -> ConfigResult<Self> {
        use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
        let (tx, rx) = channel();

        let mut watcher = RecommendedWatcher::new(tx, Config::default())
            .map_err(|e| ConfigError::WatchFailed(e.to_string()))?;

        watcher
            .watch(dir, RecursiveMode::NonRecursive)
            .map_err(|e| ConfigError::WatchFailed(e.to_string()))?;

        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }

    /// Check if any configuration files were modified since last check.
    pub fn poll_changes(&self) -> bool {
        let mut changed = false;
        while let Ok(event) = self.rx.try_recv() {
            if let Ok(ev) = event {
                if ev.kind.is_modify() || ev.kind.is_create() {
                    changed = true;
                }
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_loader_default_creation_and_preservation() {
        let dir = tempdir().unwrap();
        let settings = ConfigLoader::load_settings(dir.path()).unwrap();
        assert_eq!(settings.version, CURRENT_SCHEMA_VERSION);
        assert!(settings.directories_first);
        assert!(!settings.show_hidden);

        let keymap = ConfigLoader::load_keymap(dir.path()).unwrap();
        assert_eq!(keymap.version, CURRENT_SCHEMA_VERSION);

        let theme = ConfigLoader::load_theme(dir.path()).unwrap();
        assert_eq!(theme.name, "default-dark");
    }

    #[test]
    fn test_config_migration_runner_v0_to_v1() {
        let dir = tempdir().unwrap();
        let settings_path = dir.path().join("settings.toml");

        // Write old v0 settings.toml without version field and with custom key
        let v0_content = r#"
show_hidden = true
custom_user_plugin_setting = "enabled"
"#;
        fs::write(&settings_path, v0_content).unwrap();

        // Load settings -> should trigger migration v0 -> v1
        let settings = ConfigLoader::load_settings(dir.path()).unwrap();

        assert_eq!(settings.version, 1);
        assert!(settings.show_hidden);
        // Verify unknown key survived rewrite in extra_fields!
        assert!(settings.extra_fields.contains_key("custom_user_plugin_setting"));

        // Verify backup file settings.toml.v0.bak was created
        let backup_path = dir.path().join("settings.toml.v0.bak");
        assert!(backup_path.exists());
        let backup_content = fs::read_to_string(&backup_path).unwrap();
        assert_eq!(backup_content, v0_content);
    }
}
