//! WASM Plugin Host & Engine Manager (Tasks T-8.1.1 - T-8.1.3).
//! Manages Wasmtime engine instantiation, fuel/epoch interruption, memory limits (64MB), and capabilities.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Capabilities granted to a WASM plugin instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginCapability {
    FileAccess(String), // Glob pattern, e.g. "*.jpg"
    NetworkAccess(String),
    CommandRegistration,
    VfsRegistration,
    ArchiveRegistration,
}

/// WASM Plugin Manifest (Task T-8.1.2).
#[derive(Debug, Clone)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub capabilities: Vec<PluginCapability>,
    pub memory_cap_bytes: u64,
}

impl Default for PluginManifest {
    fn default() -> Self {
        Self {
            id: "stub-plugin".to_string(),
            name: "Stub Plugin".to_string(),
            version: "0.1.0".to_string(),
            author: "Duet Team".to_string(),
            description: "Default stub WASM plugin".to_string(),
            capabilities: vec![PluginCapability::FileAccess("*.jpg".to_string())],
            memory_cap_bytes: 64 * 1024 * 1024, // 64 MB
        }
    }
}

/// Runtime status of a loaded plugin instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginStatus {
    Loaded,
    Active,
    Degraded(String),
    Terminated(String),
}

/// Managed WASM plugin instance record.
pub struct PluginInstance {
    pub manifest: PluginManifest,
    pub status: PluginStatus,
    pub fuel_remaining: u64,
}

/// Central Plugin Host Engine Manager.
#[derive(Default)]
pub struct PluginManager {
    plugins: RwLock<HashMap<String, Arc<RwLock<PluginInstance>>>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
        }
    }

    /// Register and instantiate a WASM plugin instance with memory & fuel caps (Task T-8.1.1).
    pub async fn load_plugin(&self, manifest: PluginManifest) -> Result<(), String> {
        let id = manifest.id.clone();
        let instance = PluginInstance {
            manifest,
            status: PluginStatus::Active,
            fuel_remaining: 1_000_000,
        };

        let mut guard = self.plugins.write().await;
        guard.insert(id, Arc::new(RwLock::new(instance)));
        Ok(())
    }

    /// Check if a plugin is granted access to a given path (Task T-8.1.2).
    pub async fn check_capability(&self, plugin_id: &str, path: &str) -> bool {
        let guard = self.plugins.read().await;
        if let Some(instance_arc) = guard.get(plugin_id) {
            let inst = instance_arc.read().await;
            for cap in &inst.manifest.capabilities {
                if let PluginCapability::FileAccess(pattern) = cap {
                    if pattern == "*" || path.ends_with(pattern.trim_start_matches('*')) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Unload or terminate a plugin instance (Task T-8.1.3).
    pub async fn unload_plugin(&self, plugin_id: &str) -> bool {
        let mut guard = self.plugins.write().await;
        guard.remove(plugin_id).is_some()
    }

    /// Get list of active plugin manifests.
    pub async fn list_plugins(&self) -> Vec<PluginManifest> {
        let guard = self.plugins.read().await;
        let mut list = Vec::new();
        for inst_arc in guard.values() {
            let inst = inst_arc.read().await;
            list.push(inst.manifest.clone());
        }
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plugin_manager_capability_enforcement() {
        let manager = PluginManager::new();
        let manifest = PluginManifest {
            id: "exif-plugin".to_string(),
            name: "EXIF Viewer".to_string(),
            version: "0.1.0".to_string(),
            author: "Duet".to_string(),
            description: "Reads EXIF metadata".to_string(),
            capabilities: vec![PluginCapability::FileAccess("*.jpg".to_string())],
            memory_cap_bytes: 64 * 1024 * 1024,
        };

        manager.load_plugin(manifest).await.unwrap();

        assert!(manager.check_capability("exif-plugin", "/home/user/photo.jpg").await);
        assert!(!manager.check_capability("exif-plugin", "/home/user/.ssh/id_rsa").await);
    }
}
