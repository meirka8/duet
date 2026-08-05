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
    Mode,
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

/// Natural numeric comparison helper for filenames ("file2.txt" < "file10.txt").
pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(&ca), Some(&cb)) => {
                if ca.is_ascii_digit() && cb.is_ascii_digit() {
                    let mut num_a: u64 = 0;
                    while let Some(&c) = a_chars.peek() {
                        if let Some(digit) = c.to_digit(10) {
                            num_a = num_a.saturating_mul(10).saturating_add(digit as u64);
                            a_chars.next();
                        } else {
                            break;
                        }
                    }

                    let mut num_b: u64 = 0;
                    while let Some(&c) = b_chars.peek() {
                        if let Some(digit) = c.to_digit(10) {
                            num_b = num_b.saturating_mul(10).saturating_add(digit as u64);
                            b_chars.next();
                        } else {
                            break;
                        }
                    }

                    match num_a.cmp(&num_b) {
                        std::cmp::Ordering::Equal => continue,
                        non_eq => return non_eq,
                    }
                } else {
                    let ca_lower = ca.to_lowercase().next().unwrap_or(ca);
                    let cb_lower = cb.to_lowercase().next().unwrap_or(cb);
                    match ca_lower.cmp(&cb_lower) {
                        std::cmp::Ordering::Equal => {
                            if ca != cb {
                                return ca.cmp(&cb);
                            }
                            a_chars.next();
                            b_chars.next();
                        }
                        non_eq => return non_eq,
                    }
                }
            }
        }
    }
}

/// Glob wildcard pattern matching (* and ? support).
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern_lower = pattern.to_lowercase();
    let text_lower = text.to_lowercase();

    let p_bytes = pattern_lower.as_bytes();
    let t_bytes = text_lower.as_bytes();

    let mut px = 0;
    let mut tx = 0;
    let mut next_px = 0;
    let mut next_tx = 0;

    while px < p_bytes.len() || tx < t_bytes.len() {
        if px < p_bytes.len() {
            let c = p_bytes[px];
            match c {
                b'?' => {
                    if tx < t_bytes.len() {
                        px += 1;
                        tx += 1;
                        continue;
                    }
                }
                b'*' => {
                    next_px = px + 1;
                    next_tx = tx + 1;
                    px += 1;
                    continue;
                }
                _ => {
                    if tx < t_bytes.len() && t_bytes[tx] == c {
                        px += 1;
                        tx += 1;
                        continue;
                    }
                }
            }
        }

        if next_px > 0 && next_tx <= t_bytes.len() {
            px = next_px;
            tx = next_tx;
            next_tx += 1;
            continue;
        }

        return false;
    }

    true
}

/// Directory panel view model managing entry layout, sorting, filtering, selection, and debounced file watching notifications.
#[derive(Debug)]
pub struct DirectoryModel {
    store: EntryStore,
    view_indices: Vec<usize>,
    selection: HashSet<EntryId>,
    sort_column: SortColumn,
    secondary_sort_column: SortColumn,
    sort_direction: SortDirection,
    directories_first: bool,
    natural_sort: bool,
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
            secondary_sort_column: SortColumn::Name,
            sort_direction: SortDirection::Ascending,
            directories_first: true,
            natural_sort: true,
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
        if self.sort_column != column {
            self.secondary_sort_column = self.sort_column;
            self.sort_column = column;
        }
        self.sort_direction = direction;
        self.rebuild_view()
    }

    pub fn set_natural_sort(&mut self, natural: bool) -> DiffBatch {
        self.natural_sort = natural;
        self.rebuild_view()
    }

    pub fn set_directories_first(&mut self, dirs_first: bool) -> DiffBatch {
        self.directories_first = dirs_first;
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

    pub fn select_by_pattern(&mut self, mask: &str) {
        for &idx in &self.view_indices {
            let name = self.store.get_name(idx);
            if glob_match(mask, name) {
                self.selection.insert(self.store.id(idx));
            }
        }
    }

    pub fn select_by_extension(&mut self, ext: &str) {
        let pattern = format!("*.{ext}");
        self.select_by_pattern(&pattern);
    }

    pub fn invert_selection(&mut self) {
        for &idx in &self.view_indices {
            let id = self.store.id(idx);
            if self.selection.contains(&id) {
                self.selection.remove(&id);
            } else {
                self.selection.insert(id);
            }
        }
    }

    pub fn selection_stats(&self) -> (usize, u64) {
        let mut count = 0;
        let mut total_size = 0;
        for i in 0..self.store.len() {
            if self.selection.contains(&self.store.id(i)) {
                count += 1;
                total_size += self.store.size(i);
            }
        }
        (count, total_size)
    }

    pub fn push_watch_event(&mut self, event: WatchEvent) {
        self.watch_buffer.push((Instant::now(), event));
    }

    /// Process debounced file watching events, applying mutations and producing minimal `DiffBatch` updates.
    pub fn process_watch_events(&mut self, now: Instant) -> Vec<DiffBatch> {
        let mut diffs = Vec::new();

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
            if !mask.is_empty() && mask != "*" && !glob_match(mask, name) {
                return false;
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

            let cmp_primary = self.compare_by_column(a, b, self.sort_column);
            let cmp = if cmp_primary == std::cmp::Ordering::Equal {
                self.compare_by_column(a, b, self.secondary_sort_column)
            } else {
                cmp_primary
            };

            match self.sort_direction {
                SortDirection::Ascending => cmp,
                SortDirection::Descending => cmp.reverse(),
            }
        });
    }

    fn compare_by_column(&self, a: usize, b: usize, col: SortColumn) -> std::cmp::Ordering {
        match col {
            SortColumn::Name => {
                if self.natural_sort {
                    natural_cmp(self.store.get_name(a), self.store.get_name(b))
                } else {
                    self.store.get_name(a).cmp(self.store.get_name(b))
                }
            }
            SortColumn::Size => self.store.size(a).cmp(&self.store.size(b)),
            SortColumn::Mtime => self.store.mtime(a).cmp(&self.store.mtime(b)),
            SortColumn::FileType => (self.store.file_type(a) as u8).cmp(&(self.store.file_type(b) as u8)),
            SortColumn::Mode => self.store.get(a).mode.cmp(&self.store.get(b).mode),
        }
    }
}
