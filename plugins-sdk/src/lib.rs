//! Duet Plugin SDK
//! 
//! Provides Rust WIT bindings and helper types for writing Duet file manager plugins.
//! Duet plugins run as WASM components under zero ambient authority, communicating
//! exclusively through host-granted capability handles and host functions.

wit_bindgen::generate!({
    world: "duet-plugin",
    path: "wit",
});

pub mod prelude {
    pub use crate::duet::plugin::host::{
        self, Entry, Error, ErrorCode, LogLevel,
    };
    pub use crate::exports::duet::plugin::command_plugin::{
        CommandInfo, Guest as CommandPluginGuest, SelectionContext,
    };
    pub use crate::exports::duet::plugin::content_plugin::{
        FieldDef, FieldType, FieldValue, Guest as ContentPluginGuest,
    };
    pub use crate::exports::duet::plugin::fs_plugin::{
        Guest as FsPluginGuest,
    };
    pub use crate::exports::duet::plugin::packer_plugin::{
        Guest as PackerPluginGuest,
    };
    pub use crate::exports::duet::plugin::viewer_plugin::{
        CanvasData, Guest as ViewerPluginGuest, RenderTarget,
    };
}
