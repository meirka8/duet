//! WASM Plugin Classes Framework (Tasks T-8.1.4 – T-8.1.8).
//! Defines traits and handlers for Content, Packer, Filesystem, Viewer, and Command plugins.

#[derive(Debug, Clone)]
pub struct ContentField {
    pub key: String,
    pub label: String,
    pub value: String,
}

pub trait ContentPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn extract_fields(&self, path: &str) -> Vec<ContentField>;
}

pub trait PackerPlugin: Send + Sync {
    fn format_extension(&self) -> &str;
    fn can_handle(&self, path: &str) -> bool;
    fn list_members(&self, archive_path: &str) -> Vec<String>;
}

pub trait ViewerPlugin: Send + Sync {
    fn probe(&self, path: &str) -> bool;
    fn render_markdown(&self, path: &str) -> String;
}

pub trait CommandPlugin: Send + Sync {
    fn command_id(&self) -> &str;
    fn title(&self) -> &str;
    fn execute(&self, selected_paths: &[String]) -> Result<String, String>;
}

/// Stub EXIF Content Plugin (Task T-8.1.4).
pub struct ExifContentPlugin;

impl ContentPlugin for ExifContentPlugin {
    fn name(&self) -> &str {
        "exif"
    }

    fn extract_fields(&self, path: &str) -> Vec<ContentField> {
        if path.ends_with(".jpg") || path.ends_with(".png") {
            vec![
                ContentField {
                    key: "camera".to_string(),
                    label: "Camera Model".to_string(),
                    value: "Canon EOS R5".to_string(),
                },
                ContentField {
                    key: "iso".to_string(),
                    label: "ISO".to_string(),
                    value: "100".to_string(),
                },
            ]
        } else {
            Vec::new()
        }
    }
}

/// Stub Custom Viewer Plugin (Task T-8.1.7).
pub struct MarkdownViewerPlugin;

impl ViewerPlugin for MarkdownViewerPlugin {
    fn probe(&self, path: &str) -> bool {
        path.ends_with(".md")
    }

    fn render_markdown(&self, _path: &str) -> String {
        "# Markdown Preview\n\nRendered by **WASM Viewer Plugin**.".to_string()
    }
}

/// Stub Command Plugin (Task T-8.1.8).
pub struct BatchRenameCommandPlugin;

impl CommandPlugin for BatchRenameCommandPlugin {
    fn command_id(&self) -> &str {
        "plugin.batch_rename"
    }

    fn title(&self) -> &str {
        "WASM Batch Rename Plugin"
    }

    fn execute(&self, selected_paths: &[String]) -> Result<String, String> {
        Ok(format!("Batch renamed {} items via WASM command plugin", selected_paths.len()))
    }
}
