use notify::{Config, Event, EventKind, PollWatcher, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

/// Event payload emitted by the filesystem watcher after debouncing and coalescing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoalescedWatchEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
    RescanRequired,
}

/// Filesystem directory watching service with debouncing, event coalescing, overflow detection, and polling fallback.
pub struct DirectoryWatcher {
    raw_rx: Receiver<notify::Result<Event>>,
    _recommended_watcher: Option<RecommendedWatcher>,
    _poll_watcher: Option<PollWatcher>,
    debounce_duration: Duration,
    pending_events: HashMap<PathBuf, (Instant, CoalescedWatchEvent)>,
    rescan_requested: bool,
}

impl DirectoryWatcher {
    pub fn new(path: &Path, is_remote_or_nfs: bool) -> notify::Result<Self> {
        let (tx, rx) = channel();

        let (recommended_watcher, poll_watcher) = if is_remote_or_nfs {
            let config = Config::default().with_poll_interval(Duration::from_secs(2));
            let mut watcher = PollWatcher::new(tx, config)?;
            watcher.watch(path, RecursiveMode::NonRecursive)?;
            (None, Some(watcher))
        } else {
            let tx_clone = tx.clone();
            match RecommendedWatcher::new(tx_clone, Config::default()) {
                Ok(mut watcher) => {
                    if let Err(e) = watcher.watch(path, RecursiveMode::NonRecursive) {
                        log::warn!("RecommendedWatcher failed for path {:?}: {}, falling back to PollWatcher", path, e);
                        let config = Config::default().with_poll_interval(Duration::from_secs(2));
                        let mut poll = PollWatcher::new(tx, config)?;
                        poll.watch(path, RecursiveMode::NonRecursive)?;
                        (None, Some(poll))
                    } else {
                        (Some(watcher), None)
                    }
                }
                Err(_) => {
                    let config = Config::default().with_poll_interval(Duration::from_secs(2));
                    let mut poll = PollWatcher::new(tx, config)?;
                    poll.watch(path, RecursiveMode::NonRecursive)?;
                    (None, Some(poll))
                }
            }
        };

        Ok(Self {
            raw_rx: rx,
            _recommended_watcher: recommended_watcher,
            _poll_watcher: poll_watcher,
            debounce_duration: Duration::from_millis(50),
            pending_events: HashMap::new(),
            rescan_requested: false,
        })
    }

    /// Poll and process raw watcher events, applying debouncing and coalescing.
    pub fn poll_events(&mut self) -> Vec<CoalescedWatchEvent> {
        let now = Instant::now();

        // Drain available raw events from notify receiver
        while let Ok(raw) = self.raw_rx.try_recv() {
            match raw {
                Ok(event) => {
                    if event.need_rescan() {
                        self.rescan_requested = true;
                        continue;
                    }

                    for path in event.paths {
                        match event.kind {
                            EventKind::Create(_) => {
                                self.pending_events.insert(path.clone(), (now, CoalescedWatchEvent::Created(path)));
                            }
                            EventKind::Modify(_) => {
                                // If entry is not already Created, set/update to Modified
                                self.pending_events.entry(path.clone()).or_insert((now, CoalescedWatchEvent::Modified(path)));
                            }
                            EventKind::Remove(_) => {
                                self.pending_events.insert(path.clone(), (now, CoalescedWatchEvent::Deleted(path)));
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Watch error received: {}, triggering directory rescan", e);
                    self.rescan_requested = true;
                }
            }
        }

        let mut ready = Vec::new();

        if self.rescan_requested {
            self.rescan_requested = false;
            self.pending_events.clear();
            ready.push(CoalescedWatchEvent::RescanRequired);
            return ready;
        }

        // Emit events that have exceeded the 50 ms debounce duration
        let mut keys_to_remove = Vec::new();
        for (path, (timestamp, event)) in &self.pending_events {
            if now.duration_since(*timestamp) >= self.debounce_duration {
                ready.push(event.clone());
                keys_to_remove.push(path.clone());
            }
        }

        for k in keys_to_remove {
            self.pending_events.remove(&k);
        }

        ready
    }
}
