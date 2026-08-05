use duet_index::EntryStore;
use duet_types::{EntryId, Metadata, VPath};
use duet_vfs::{FileSystem, ListOpts};
use glob::Pattern;
use regex::Regex;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Criteria for file search query.
#[derive(Debug, Clone, Default)]
pub struct SearchQuery {
    pub root_path: VPath,
    pub name_mask: Option<String>,
    pub name_regex: Option<String>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub min_mtime: Option<i64>,
    pub max_mtime: Option<i64>,
    pub content_pattern: Option<String>,
    pub case_sensitive: bool,
    pub one_filesystem: bool,
}

/// Individual search result match item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub path: VPath,
    pub metadata: Metadata,
    pub match_line: Option<usize>,
    pub match_snippet: Option<String>,
}

/// Parallel / async search engine producing streaming result channels.
#[derive(Debug, Default)]
pub struct SearchEngine;

impl SearchEngine {
    pub fn new() -> Self {
        Self
    }

    /// Execute search query streaming results via an mpsc channel.
    pub fn search_stream(
        &self,
        query: SearchQuery,
        fs: Arc<dyn FileSystem>,
    ) -> mpsc::Receiver<SearchResult> {
        let (tx, rx) = mpsc::channel(100);

        tokio::spawn(async move {
            let glob_pattern = query
                .name_mask
                .as_ref()
                .and_then(|m| Pattern::new(m).ok());

            let regex_pattern = query.name_regex.as_ref().and_then(|r| {
                if query.case_sensitive {
                    Regex::new(r).ok()
                } else {
                    Regex::new(&format!("(?i){r}")).ok()
                }
            });

            let content_regex = query.content_pattern.as_ref().and_then(|cp| {
                if query.case_sensitive {
                    Regex::new(cp).ok()
                } else {
                    Regex::new(&format!("(?i){cp}")).ok()
                }
            });

            let root_meta = fs.stat(&query.root_path, false).await;
            let root_dev = root_meta.as_ref().map(|m| m.dev).unwrap_or(0);

            let mut stack = vec![query.root_path.clone()];

            while let Some(current_dir) = stack.pop() {
                use futures::StreamExt;
                let opts = ListOpts {
                    size: true,
                    mtime: true,
                    mode: true,
                    file_type: true,
                };
                let mut stream = fs.read_dir(&current_dir, opts);

                while let Some(chunk_res) = stream.next().await {
                    let chunk = match chunk_res {
                        Ok(c) => c,
                        Err(_) => continue,
                    };

                    for entry in chunk {
                        let child_vpath = join_vpath(&current_dir, &entry.name);
                        let meta = if let Some(m) = entry.metadata {
                            m
                        } else if let Ok(m) = fs.stat(&child_vpath, false).await {
                            m
                        } else {
                            continue;
                        };

                        if query.one_filesystem && root_dev != 0 && meta.dev != 0 && meta.dev != root_dev {
                            continue;
                        }

                        if meta.is_dir() {
                            stack.push(child_vpath.clone());
                        }

                        // Evaluate name glob mask
                        let name = &entry.name;
                        if let Some(ref glob) = glob_pattern {
                            if !glob.matches(name) {
                                continue;
                            }
                        }

                        // Evaluate name regex
                        if let Some(ref re) = regex_pattern {
                            if !re.is_match(name) {
                                continue;
                            }
                        }

                        // Evaluate size filter
                        if let Some(min_s) = query.min_size {
                            if meta.size < min_s {
                                continue;
                            }
                        }
                        if let Some(max_s) = query.max_size {
                            if meta.size > max_s {
                                continue;
                            }
                        }

                        // Evaluate mtime filter
                        if let Some(min_t) = query.min_mtime {
                            if meta.modified.unwrap_or(0) < min_t {
                                continue;
                            }
                        }
                        if let Some(max_t) = query.max_mtime {
                            if meta.modified.unwrap_or(0) > max_t {
                                continue;
                            }
                        }

                        // Evaluate content pattern search if requested
                        let mut match_line = None;
                        let mut match_snippet = None;

                        if let Some(ref creg) = content_regex {
                            if meta.is_file() && child_vpath.scheme == "file" {
                                if let Ok(content) = std::fs::read_to_string(&child_vpath.path) {
                                    for (line_num, line) in (1..).zip(content.lines()) {
                                        if creg.is_match(line) {
                                            match_line = Some(line_num);
                                            match_snippet = Some(line.trim().to_string());
                                            break;
                                        }
                                    }
                                    if match_line.is_none() {
                                        continue;
                                    }
                                } else {
                                    continue;
                                }
                            } else {
                                continue;
                            }
                        }

                        let item = SearchResult {
                            path: child_vpath,
                            metadata: meta,
                            match_line,
                            match_snippet,
                        };

                        if tx.send(item).await.is_err() {
                            return;
                        }
                    }
                }
            }
        });

        rx
    }
}

/// Convert search results into a synthetic EntryStore for rendering and operating in panel views.
pub fn feed_to_panel(results: &[SearchResult]) -> EntryStore {
    let mut store = EntryStore::with_capacity(results.len());

    for (idx, item) in results.iter().enumerate() {
        let entry_id = EntryId((idx + 1) as u64);
        let name = &item.path.path;
        let meta = &item.metadata;

        store.push(
            entry_id,
            name,
            meta.file_type,
            meta.size,
            meta.mode,
            meta.uid,
            meta.gid,
            meta.modified.unwrap_or(0),
            meta.accessed.unwrap_or(0),
            meta.created.unwrap_or(0),
            meta.dev,
            meta.ino,
            meta.nlink as u32,
            0,
        );
    }

    store
}

fn join_vpath(base: &VPath, child: &str) -> VPath {
    let mut new_vpath = base.clone();
    if new_vpath.path.ends_with('/') {
        new_vpath.path.push_str(child);
    } else {
        new_vpath.path.push('/');
        new_vpath.path.push_str(child);
    }
    new_vpath
}

#[cfg(test)]
mod tests {
    use super::*;
    use duet_vfs::LocalFs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_search_engine_glob_and_content_matching() {
        duet_platform::set_ui_thread(false);
        let temp = tempdir().unwrap();
        let file1 = temp.path().join("match_one.txt");
        let file2 = temp.path().join("other.log");
        let file3 = temp.path().join("match_two.txt");

        std::fs::write(&file1, "target keyword content line").unwrap();
        std::fs::write(&file2, "unrelated text").unwrap();
        std::fs::write(&file3, "another target keyword match").unwrap();

        let fs = Arc::new(LocalFs::new());
        let engine = SearchEngine::new();

        let query = SearchQuery {
            root_path: VPath::new_local(temp.path().to_str().unwrap()),
            name_mask: Some("*.txt".to_string()),
            content_pattern: Some("keyword".to_string()),
            ..Default::default()
        };

        let mut rx = engine.search_stream(query, fs);
        let mut matches = Vec::new();
        while let Some(res) = rx.recv().await {
            matches.push(res);
        }

        assert_eq!(matches.len(), 2);
        assert!(matches.iter().any(|m| m.path.path.contains("match_one.txt")));
        assert!(matches.iter().any(|m| m.path.path.contains("match_two.txt")));
        assert!(matches.iter().all(|m| m.match_line.is_some()));

        // Test feed_to_panel
        let panel_store = feed_to_panel(&matches);
        assert_eq!(panel_store.len(), 2);
    }
}
