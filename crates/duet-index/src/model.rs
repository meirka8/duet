use crate::diff::{DiffBatch, EntryDiffData};
use crate::entry_store::EntryStore;
use duet_types::{EntryId, FileType};
use std::collections::HashSet;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortColumn {
    #[default]
    Name,
    Size,
    Mtime,
    FileType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FilterSpec {
    pub show_hidden: bool,
    pub quick_filter: Option<String>,
    pub mask: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEvent {
    Created {
        id: EntryId,
        name: String,
        file_type: FileType,
        size: u64,
        mode: u32,
        uid: u32,
        gid: u32,
        mtime: i64,
        atime: i64,
        ctime: i64,
        dev: u64,
        ino: u64,
        nlink: u32,
        flags: u32,
    },
    Modified {
        id: EntryId,
        size: u64,
        mode: u32,
        mtime: i64,
        atime: i64,
        ctime: i64,
    },
    Removed {
        id: EntryId,
    },
}

#[derive(Debug, Clone)]
pub struct EntryInput {
    pub id: EntryId,
    pub name: String,
    pub file_type: FileType,
    pub size: u64,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub mtime: i64,
    pub atime: i64,
    pub ctime: i64,
    pub dev: u64,
    pub ino: u64,
    pub nlink: u32,
    pub flags: u32,
}

/// Directory panel view model managing entry layout, sorting, filtering, selection, and debounced file watching notifications.
#[derive(Debug)]
pub struct DirectoryModel {
    store: EntryStore,
    view_indices: Vec<usize>,
    selection: HashSet<EntryId>,
    sort_column: SortColumn,
    sort_direction: SortDirection,
    directories_first: bool,
    filter_spec: FilterSpec,
    watch_buffer: Vec<(Instant, WatchEvent)>,
    debounce_duration: Duration,
}

impl Default for DirectoryModel {
    fn default() -> Self {
        Self {
            store: EntryStore::new(),
            view_indices: Vec::new(),
            selection: HashSet::new(),
            sort_column: SortColumn::Name,
            sort_direction: SortDirection::Ascending,
            directories_first: true,
            filter_spec: FilterSpec::default(),
            watch_buffer: Vec::new(),
            debounce_duration: Duration::from_millis(50),
        }
    }
}

impl DirectoryModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_debounce(debounce_duration: Duration) -> Self {
        Self {
            debounce_duration,
            ..Self::default()
        }
    }

    pub fn store(&self) -> &EntryStore {
        &self.store
    }

    pub fn view_indices(&self) -> &[usize] {
        &self.view_indices
    }

    pub fn selection(&self) -> &HashSet<EntryId> {
        &self.selection
    }

    pub fn len(&self) -> usize {
        self.view_indices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.view_indices.is_empty()
    }

    pub fn set_entries(&mut self, entries: Vec<EntryInput>) -> DiffBatch {
        self.store.clear();
        self.selection.clear();

        for e in entries {
            self.store.push(
                e.id, &e.name, e.file_type, e.size, e.mode, e.uid, e.gid, e.mtime, e.atime,
                e.ctime, e.dev, e.ino, e.nlink, e.flags,
            );
        }

        self.rebuild_view()
    }

    pub fn rebuild_view(&mut self) -> DiffBatch {
        let old_indices = self.view_indices.clone();

        let mut indices: Vec<usize> = (0..self.store.len())
            .filter(|&idx| self.matches_filter(idx))
            .collect();

        self.sort_indices(&mut indices);
        self.view_indices = indices;

        if old_indices.is_empty() && !self.view_indices.is_empty() {
            DiffBatch::Reset
        } else {
            let mut mapping = Vec::new();
            for (new_pos, &store_idx) in self.view_indices.iter().enumerate() {
                if let Some(old_pos) = old_indices.iter().position(|&i| i == store_idx) {
                    if old_pos != new_pos {
                        mapping.push((old_pos, new_pos));
                    }
                }
            }
            if mapping.is_empty() && old_indices.len() == self.view_indices.len() {
                DiffBatch::Batch(vec![])
            } else {
                DiffBatch::Reset
            }
        }
    }

    pub fn sort(&mut self, column: SortColumn, direction: SortDirection) -> DiffBatch {
        self.sort_column = column;
        self.sort_direction = direction;
        self.rebuild_view()
    }

    pub fn filter(&mut self, spec: FilterSpec) -> DiffBatch {
        self.filter_spec = spec;
        self.rebuild_view()
    }

    pub fn toggle_selection(&mut self, id: EntryId) -> bool {
        if self.selection.contains(&id) {
            self.selection.remove(&id);
            false
        } else {
            self.selection.insert(id);
            true
        }
    }

    pub fn select_all(&mut self) {
        for &idx in &self.view_indices {
            self.selection.insert(self.store.id(idx));
        }
    }

    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    pub fn push_watch_event(&mut self, event: WatchEvent) {
        self.watch_buffer.push((Instant::now(), event));
    }

    /// Process debounced file watching events, applying mutations and producing minimal `DiffBatch` updates.
    pub fn process_watch_events(&mut self, now: Instant) -> Vec<DiffBatch> {
        let mut diffs = Vec::new();

        // Partition events into ready (elapsed >= debounce_duration) vs pending
        let mut ready_events = Vec::new();
        let mut remaining = Vec::new();

        for (ts, ev) in self.watch_buffer.drain(..) {
            if now.duration_since(ts) >= self.debounce_duration {
                ready_events.push(ev);
            } else {
                remaining.push((ts, ev));
            }
        }
        self.watch_buffer = remaining;

        for ev in ready_events {
            match ev {
                WatchEvent::Created {
                    id,
                    name,
                    file_type,
                    size,
                    mode,
                    uid,
                    gid,
                    mtime,
                    atime,
                    ctime,
                    dev,
                    ino,
                    nlink,
                    flags,
                } => {
                    let idx = self.store.push(
                        id, &name, file_type, size, mode, uid, gid, mtime, atime, ctime, dev,
                        ino, nlink, flags,
                    );
                    if self.matches_filter(idx) {
                        self.view_indices.push(idx);
                        let mut new_indices = self.view_indices.clone();
                        self.sort_indices(&mut new_indices);
                        let new_pos = new_indices.iter().position(|&i| i == idx).unwrap_or(0);
                        self.view_indices = new_indices;

                        diffs.push(DiffBatch::Insert {
                            index: new_pos,
                            entry: EntryDiffData {
                                id,
                                name,
                                file_type,
                                size,
                                mtime,
                            },
                        });
                    }
                }
                WatchEvent::Modified {
                    id,
                    size,
                    mode,
                    mtime,
                    atime,
                    ctime,
                } => {
                    if let Some(store_idx) = (0..self.store.len()).find(|&i| self.store.id(i) == id) {
                        self.store
                            .update_entry(store_idx, size, mode, mtime, atime, ctime);
                        if let Some(view_pos) = self.view_indices.iter().position(|&i| i == store_idx) {
                            diffs.push(DiffBatch::Update {
                                index: view_pos,
                                entry: EntryDiffData {
                                    id,
                                    name: self.store.get_name(store_idx).to_string(),
                                    file_type: self.store.file_type(store_idx),
                                    size,
                                    mtime,
                                },
                            });
                        }
                    }
                }
                WatchEvent::Removed { id } => {
                    self.selection.remove(&id);
                    if let Some(store_idx) = (0..self.store.len()).find(|&i| self.store.id(i) == id) {
                        if let Some(view_pos) = self.view_indices.iter().position(|&i| i == store_idx) {
                            self.view_indices.remove(view_pos);
                            diffs.push(DiffBatch::Remove {
                                index: view_pos,
                                id,
                            });
                        }
                    }
                }
            }
        }

        diffs
    }

    fn matches_filter(&self, index: usize) -> bool {
        let name = self.store.get_name(index);

        if !self.filter_spec.show_hidden && name.starts_with('.') {
            return false;
        }

        if let Some(ref q) = self.filter_spec.quick_filter {
            if !q.is_empty() && !name.to_lowercase().contains(&q.to_lowercase()) {
                return false;
            }
        }

        if let Some(ref mask) = self.filter_spec.mask {
            if !mask.is_empty() && mask != "*" {
                let pattern = mask.replace('*', ".*");
                if let Ok(re) = regex_lite_match(&pattern, name) {
                    if !re {
                        return false;
                    }
                }
            }
        }

        true
    }

    fn sort_indices(&self, indices: &mut [usize]) {
        indices.sort_by(|&a, &b| {
            let type_a = self.store.file_type(a);
            let type_b = self.store.file_type(b);

            if self.directories_first {
                if type_a.is_dir() && !type_b.is_dir() {
                    return std::cmp::Ordering::Less;
                }
                if !type_a.is_dir() && type_b.is_dir() {
                    return std::cmp::Ordering::Greater;
                }
            }

            let cmp = match self.sort_column {
                SortColumn::Name => self.store.get_name(a).cmp(self.store.get_name(b)),
                SortColumn::Size => self.store.size(a).cmp(&self.store.size(b)),
                SortColumn::Mtime => self.store.mtime(a).cmp(&self.store.mtime(b)),
                SortColumn::FileType => (type_a as u8).cmp(&(type_b as u8)),
            };

            match self.sort_direction {
                SortDirection::Ascending => cmp,
                SortDirection::Descending => cmp.reverse(),
            }
        });
    }
}

fn regex_lite_match(pattern: &str, text: &str) -> Result<bool, ()> {
    if let Some(sub) = pattern.strip_prefix(".*") {
        Ok(text.ends_with(sub) || text.contains(sub))
    } else {
        Ok(text.contains(pattern))
    }
}
