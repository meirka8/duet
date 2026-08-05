//! CI Lint: GPUI Dependency Isolation Check (ADR-002 / T-2.1.2)
//!
//! Asserts that ONLY `duet-ui`, `duet-widgets`, and application shell (`duet`)
//! are permitted to depend on `gpui` or `gpui-component`.
//! Core crates (`duet-vfs`, `duet-ops`, `duet-types`, `duet-index`, `duet-search`,
//! `duet-meta`, `duet-commands`, `duet-config`, `duet-plugin`, `duet-platform`, etc.)
//! must remain completely UI-agnostic.

use std::fs;
use std::path::{Path, PathBuf};

/// Crates allowed to depend on GPUI ecosystem dependencies.
pub const ALLOWED_UI_CRATES: &[&str] = &[
    "duet-ui",
    "duet-widgets",
    "duet",
];

/// Forbidden GPUI ecosystem dependency names for core crates.
pub const FORBIDDEN_GPUI_DEPS: &[&str] = &[
    "gpui",
    "gpui-component",
    "gpui-macros",
    "gpui_util",
    "gpui_sum_tree",
    "gpui_refineable",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsolationViolation {
    pub crate_name: String,
    pub forbidden_dep: String,
    pub manifest_path: PathBuf,
    pub section: String,
}

/// Inspects the workspace crates and asserts that non-UI crates do not depend on GPUI.
pub fn check_workspace_gpui_isolation(workspace_root: &Path) -> Result<(), Vec<IsolationViolation>> {
    let mut all_violations = Vec::new();
    let members = find_workspace_crates(workspace_root);

    for (crate_name, manifest_path) in members {
        if ALLOWED_UI_CRATES.contains(&crate_name.as_str()) {
            continue;
        }

        if let Ok(content) = fs::read_to_string(&manifest_path) {
            let violations = inspect_manifest_content(&crate_name, &manifest_path, &content);
            all_violations.extend(violations);
        }
    }

    if all_violations.is_empty() {
        Ok(())
    } else {
        Err(all_violations)
    }
}

/// Locates all workspace member crate manifest paths.
pub fn find_workspace_crates(workspace_root: &Path) -> Vec<(String, PathBuf)> {
    let mut results = Vec::new();
    let crates_dir = workspace_root.join("crates");
    if crates_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&crates_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let manifest = path.join("Cargo.toml");
                if manifest.is_file() {
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    results.push((name, manifest));
                }
            }
        }
    }
    let plugins_sdk_dir = workspace_root.join("plugins-sdk");
    if plugins_sdk_dir.is_dir() {
        let manifest = plugins_sdk_dir.join("Cargo.toml");
        if manifest.is_file() {
            results.push(("plugins-sdk".to_string(), manifest));
        }
    }
    results
}

/// Inspects the content of a single Cargo.toml file for forbidden GPUI dependencies.
pub fn inspect_manifest_content(
    crate_name: &str,
    manifest_path: &Path,
    content: &str,
) -> Vec<IsolationViolation> {
    let mut violations = Vec::new();
    let mut current_section = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = trimmed[1..trimmed.len() - 1].to_string();
            continue;
        }

        if current_section.contains("dependencies") {
            for &forbidden in FORBIDDEN_GPUI_DEPS {
                if is_dependency_declaration(trimmed, forbidden) {
                    violations.push(IsolationViolation {
                        crate_name: crate_name.to_string(),
                        forbidden_dep: forbidden.to_string(),
                        manifest_path: manifest_path.to_path_buf(),
                        section: current_section.clone(),
                    });
                }
            }
        }
    }

    violations
}

fn is_dependency_declaration(line: &str, dep_name: &str) -> bool {
    let key = line
        .split('=')
        .next()
        .unwrap_or("")
        .trim()
        .trim_matches('"')
        .trim_matches('\'');
    key == dep_name
}

pub fn find_workspace_root_dir(start: &Path) -> PathBuf {
    let mut curr = start.to_path_buf();
    loop {
        let cargo_path = curr.join("Cargo.toml");
        if cargo_path.exists() {
            if let Ok(content) = fs::read_to_string(&cargo_path) {
                if content.contains("[workspace]") {
                    return curr;
                }
            }
        }
        if !curr.pop() {
            return start.to_path_buf();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_workspace_gpui_isolation() {
        let current_dir = std::env::current_dir().unwrap();
        let root = find_workspace_root_dir(&current_dir);
        let result = check_workspace_gpui_isolation(&root);
        assert!(
            result.is_ok(),
            "Workspace GPUI isolation check failed with violations: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_deliberate_mock_violation_fails() {
        let mock_manifest = r#"
            [package]
            name = "duet-vfs"
            version = "0.1.0"

            [dependencies]
            gpui = "0.2.2"
            tokio = { workspace = true }
        "#;

        let violations = inspect_manifest_content(
            "duet-vfs",
            Path::new("/mock/crates/duet-vfs/Cargo.toml"),
            mock_manifest,
        );

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].crate_name, "duet-vfs");
        assert_eq!(violations[0].forbidden_dep, "gpui");
        assert_eq!(violations[0].section, "dependencies");
    }
}
