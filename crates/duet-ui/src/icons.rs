//! Icon rendering with fallback XDG extension mapping.

use duet_types::FileType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconCategory {
    Folder,
    ParentDir,
    Audio,
    Video,
    Image,
    Archive,
    Code,
    Document,
    Executable,
    Symlink,
    DefaultFile,
}

impl IconCategory {
    pub fn glyph(&self) -> &'static str {
        match self {
            IconCategory::Folder => "📁",
            IconCategory::ParentDir => "⬆️",
            IconCategory::Audio => "🎵",
            IconCategory::Video => "🎬",
            IconCategory::Image => "🖼️",
            IconCategory::Archive => "📦",
            IconCategory::Code => "📄",
            IconCategory::Document => "📑",
            IconCategory::Executable => "⚙️",
            IconCategory::Symlink => "🔗",
            IconCategory::DefaultFile => "📄",
        }
    }
}

/// Map file extension and file type to an icon category.
pub fn resolve_icon(name: &str, file_type: FileType) -> IconCategory {
    if name == ".." {
        return IconCategory::ParentDir;
    }

    match file_type {
        FileType::Directory => IconCategory::Folder,
        FileType::Symlink => IconCategory::Symlink,
        FileType::File => {
            if let Some(ext) = name.rsplit('.').next() {
                match ext.to_lowercase().as_str() {
                    "mp3" | "flac" | "wav" | "ogg" | "m4a" | "aac" => IconCategory::Audio,
                    "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" => IconCategory::Video,
                    "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "bmp" => IconCategory::Image,
                    "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "zst" => IconCategory::Archive,
                    "rs" | "c" | "cpp" | "h" | "py" | "js" | "ts" | "go" | "java" | "sh" | "toml" | "json" => IconCategory::Code,
                    "pdf" | "doc" | "docx" | "txt" | "md" | "odt" => IconCategory::Document,
                    "exe" | "bin" | "app" | "elf" => IconCategory::Executable,
                    _ => IconCategory::DefaultFile,
                }
            } else {
                IconCategory::DefaultFile
            }
        }
        _ => IconCategory::DefaultFile,
    }
}
