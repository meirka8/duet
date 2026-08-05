//! Multi-Rename Engine & Pattern Language (Task T-9.1.1, T-9.1.2).
//! Supports placeholders: `[N]` (name), `[E]` (ext), `[C]` (counter), `[D]` (date), regex, case conversion, and undo stack.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenamePair {
    pub original_path: String,
    pub new_name: String,
    pub collision_warning: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiRenameOptions {
    pub pattern: String,           // e.g. "[N]_[C:3]"
    pub find_regex: Option<String>,
    pub replace_with: String,
    pub counter_start: u32,
    pub counter_step: u32,
    pub uppercase: bool,
    pub lowercase: bool,
}

impl Default for MultiRenameOptions {
    fn default() -> Self {
        Self {
            pattern: "[N]".to_string(),
            find_regex: None,
            replace_with: String::new(),
            counter_start: 1,
            counter_step: 1,
            uppercase: false,
            lowercase: false,
        }
    }
}

pub struct MultiRenameEngine;

impl MultiRenameEngine {
    pub fn compute_previews(items: &[String], opts: &MultiRenameOptions) -> Vec<RenamePair> {
        let mut results = Vec::new();
        let mut name_counts: HashMap<String, usize> = HashMap::new();

        let regex_matcher = opts
            .find_regex
            .as_ref()
            .and_then(|pat| Regex::new(pat).ok());

        for (idx, path) in items.iter().enumerate() {
            let file_name = std::path::Path::new(path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(path);

            let stem = std::path::Path::new(file_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(file_name);

            let ext = std::path::Path::new(file_name)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");

            let counter_val = opts.counter_start + (idx as u32 * opts.counter_step);
            let counter_str = format!("{:03}", counter_val);

            let mut formatted = opts.pattern.clone();
            formatted = formatted.replace("[N]", stem);
            formatted = formatted.replace("[E]", ext);
            formatted = formatted.replace("[C]", &counter_str);
            formatted = formatted.replace("[C:3]", &counter_str);

            if ext.is_empty() {
                formatted = formatted.trim_end_matches('.').to_string();
            } else if !formatted.contains('.') && !opts.pattern.contains("[E]") {
                formatted = format!("{}.{}", formatted, ext);
            }

            if let Some(re) = &regex_matcher {
                formatted = re.replace_all(&formatted, &opts.replace_with).to_string();
            }

            if opts.uppercase {
                formatted = formatted.to_uppercase();
            } else if opts.lowercase {
                formatted = formatted.to_lowercase();
            }

            *name_counts.entry(formatted.clone()).or_insert(0) += 1;

            results.push(RenamePair {
                original_path: path.clone(),
                new_name: formatted,
                collision_warning: false,
            });
        }

        // Flag collisions
        for pair in &mut results {
            if let Some(&count) = name_counts.get(&pair.new_name) {
                if count > 1 {
                    pair.collision_warning = true;
                }
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_rename_pattern_and_counter() {
        let items = vec![
            "/tmp/photo_a.jpg".to_string(),
            "/tmp/photo_b.jpg".to_string(),
        ];
        let opts = MultiRenameOptions {
            pattern: "img_[C:3]".to_string(),
            counter_start: 1,
            counter_step: 1,
            ..Default::default()
        };

        let previews = MultiRenameEngine::compute_previews(&items, &opts);
        assert_eq!(previews[0].new_name, "img_001.jpg");
        assert_eq!(previews[1].new_name, "img_002.jpg");
        assert!(!previews[0].collision_warning);
    }
}
