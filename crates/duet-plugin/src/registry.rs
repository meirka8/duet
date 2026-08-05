//! Plugin Registry Index & Installation Client (Task T-8.1.10).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::manager::PluginManifest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryIndexEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub download_url: String,
    pub sha256: String,
}

pub struct PluginRegistry {
    index: HashMap<String, RegistryIndexEntry>,
    installed: HashMap<String, PluginManifest>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    pub fn new() -> Self {
        let mut index = HashMap::new();
        index.insert(
            "exif-viewer".to_string(),
            RegistryIndexEntry {
                id: "exif-viewer".to_string(),
                name: "EXIF Column Plugin".to_string(),
                version: "1.0.0".to_string(),
                author: "Community".to_string(),
                description: "Adds EXIF camera model and ISO columns to image directories".to_string(),
                download_url: "https://registry.duet.fm/plugins/exif-viewer-1.0.0.wasm".to_string(),
                sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
            },
        );

        Self {
            index,
            installed: HashMap::new(),
        }
    }

    pub fn search(&self, query: &str) -> Vec<RegistryIndexEntry> {
        let q = query.to_lowercase();
        self.index
            .values()
            .filter(|e| e.name.to_lowercase().contains(&q) || e.description.to_lowercase().contains(&q))
            .cloned()
            .collect()
    }

    pub fn install(&mut self, plugin_id: &str) -> Result<PluginManifest, String> {
        if let Some(entry) = self.index.get(plugin_id) {
            let manifest = PluginManifest {
                id: entry.id.clone(),
                name: entry.name.clone(),
                version: entry.version.clone(),
                author: entry.author.clone(),
                description: entry.description.clone(),
                capabilities: Vec::new(),
                memory_cap_bytes: 64 * 1024 * 1024,
            };
            self.installed.insert(plugin_id.to_string(), manifest.clone());
            Ok(manifest)
        } else {
            Err(format!("Plugin '{}' not found in registry index", plugin_id))
        }
    }

    pub fn uninstall(&mut self, plugin_id: &str) -> bool {
        self.installed.remove(plugin_id).is_some()
    }

    pub fn list_installed(&self) -> Vec<PluginManifest> {
        self.installed.values().cloned().collect()
    }
}
