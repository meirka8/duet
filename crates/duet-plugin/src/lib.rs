//! WASM Plugin Infrastructure for Duet (Phase 8, Tasks T-8.1.1 – T-8.1.12).

pub mod manager;
pub mod classes;
pub mod registry;

pub use classes::{
    BatchRenameCommandPlugin, CommandPlugin, ContentField, ContentPlugin, ExifContentPlugin,
    MarkdownViewerPlugin, PackerPlugin, ViewerPlugin,
};
pub use manager::{PluginCapability, PluginInstance, PluginManager, PluginManifest, PluginStatus};
pub use registry::{PluginRegistry, RegistryIndexEntry};

pub fn init() {}
