use duet_types::VPath;

/// Clipboard action mode for file transfers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardAction {
    Copy,
    Cut,
}

/// Format GNOME copied files clipboard content (`x-special/gnome-copied-files`).
/// First line is `copy` or `cut`, followed by newline-separated file URIs.
pub fn format_gnome_copied_files(action: ClipboardAction, paths: &[VPath]) -> String {
    let mut out = String::new();
    match action {
        ClipboardAction::Copy => out.push_str("copy\n"),
        ClipboardAction::Cut => out.push_str("cut\n"),
    }

    for path in paths {
        out.push_str(&path.to_string());
        out.push('\n');
    }
    out
}

/// Parse GNOME copied files format (`x-special/gnome-copied-files`).
pub fn parse_gnome_copied_files(data: &str) -> Option<(ClipboardAction, Vec<VPath>)> {
    let mut lines = data.lines().filter(|l| !l.trim().is_empty());
    let action_line = lines.next()?;

    let action = match action_line.trim() {
        "copy" => ClipboardAction::Copy,
        "cut" => ClipboardAction::Cut,
        _ => return None,
    };

    let mut paths = Vec::new();
    for line in lines {
        if let Ok(vpath) = VPath::parse(line.trim()) {
            paths.push(vpath);
        }
    }

    Some((action, paths))
}

/// Format standard `text/uri-list` content (CRLF-separated URIs).
pub fn format_uri_list(paths: &[VPath]) -> String {
    let mut out = String::new();
    for path in paths {
        out.push_str(&path.to_string());
        out.push_str("\r\n");
    }
    out
}

/// Parse standard `text/uri-list` content.
pub fn parse_uri_list(data: &str) -> Vec<VPath> {
    let mut paths = Vec::new();
    for line in data.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Ok(vpath) = VPath::parse(trimmed) {
            paths.push(vpath);
        }
    }
    paths
}

/// Format KDE cut selection (`application/x-kde-cutselection`).
pub fn format_kde_cut_selection(action: ClipboardAction) -> String {
    match action {
        ClipboardAction::Cut => "1".to_string(),
        ClipboardAction::Copy => "0".to_string(),
    }
}

/// Parse KDE cut selection (`application/x-kde-cutselection`).
pub fn parse_kde_cut_selection(data: &str) -> ClipboardAction {
    if data.trim() == "1" {
        ClipboardAction::Cut
    } else {
        ClipboardAction::Copy
    }
}
