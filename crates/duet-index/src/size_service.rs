use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use duet_platform::assert_not_ui_thread;

/// Key for directory size cache invalidation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub dev: u64,
    pub ino: u64,
    pub mtime: i64,
}

/// Directory tree total size result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DirSizeResult {
    pub total_bytes: u64,
    pub total_files: u64,
    pub total_dirs: u64,
}

/// Cancellable directory size calculation service with `(dev, ino, mtime)` caching.
#[derive(Debug, Clone, Default)]
pub struct DirSizeService {
    cache: Arc<RwLock<HashMap<CacheKey, DirSizeResult>>>,
}

impl DirSizeService {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Invalidate cache entry for a specific directory or file.
    pub fn invalidate(&self, key: CacheKey) {
        if let Ok(mut cache) = self.cache.write() {
            cache.remove(&key);
        }
    }

    /// Clear all cached directory size results.
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }

    /// Calculate tree size asynchronously with cancellation flag and caching.
    pub async fn compute_dir_size(
        &self,
        path: PathBuf,
        dev: u64,
        ino: u64,
        mtime: i64,
        cancel_signal: Arc<AtomicBool>,
    ) -> Option<DirSizeResult> {
        assert_not_ui_thread();

        let key = CacheKey { dev, ino, mtime };
        if let Ok(cache) = self.cache.read() {
            if let Some(cached) = cache.get(&key) {
                return Some(*cached);
            }
        }

        let cache_ref = self.cache.clone();

        tokio::task::spawn_blocking(move || {
            let mut result = DirSizeResult::default();
            let mut stack = vec![path];

            while let Some(current) = stack.pop() {
                if cancel_signal.load(Ordering::Relaxed) {
                    return None;
                }

                let read_dir = match std::fs::read_dir(&current) {
                    Ok(rd) => rd,
                    Err(_) => continue,
                };

                for entry in read_dir {
                    if cancel_signal.load(Ordering::Relaxed) {
                        return None;
                    }

                    let entry = match entry {
                        Ok(e) => e,
                        Err(_) => continue,
                    };

                    let file_type = match entry.file_type() {
                        Ok(ft) => ft,
                        Err(_) => continue,
                    };

                    if file_type.is_dir() {
                        result.total_dirs += 1;
                        stack.push(entry.path());
                    } else if file_type.is_file() {
                        result.total_files += 1;
                        if let Ok(meta) = entry.metadata() {
                            result.total_bytes += meta.len();
                        }
                    }
                }
            }

            if let Ok(mut cache) = cache_ref.write() {
                cache.insert(key, result);
            }

            Some(result)
        })
        .await
        .ok()
        .flatten()
    }
}
