use anyhow::Result;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub kind: FileChangeKind,
    pub timestamp: Instant,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileChangeKind {
    Created,
    Modified,
    Removed,
}

pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    rx: Receiver<Result<Event, notify::Error>>,
    watched_paths: Arc<Mutex<HashSet<PathBuf>>>,
    debounce_duration: Duration,
    last_events: Arc<Mutex<HashMap<PathBuf, Instant>>>,
}

impl FileWatcher {
    pub fn new(paths: Vec<PathBuf>, debounce_duration: Duration) -> Result<Self> {
        let (tx, rx) = channel();
        let watched_paths = Arc::new(Mutex::new(HashSet::new()));
        let last_events = Arc::new(Mutex::new(HashMap::new()));

        // Create watcher with a custom event handler
        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            notify::Config::default(),
        )?;

        // Watch all provided paths
        for path in &paths {
            watcher.watch(path, RecursiveMode::Recursive)?;
            watched_paths.lock().unwrap().insert(path.clone());
        }

        Ok(Self {
            _watcher: watcher,
            rx,
            watched_paths,
            debounce_duration,
            last_events,
        })
    }

    /// Check for file system events and return changed JSONL files
    pub fn poll_changes(&self) -> Vec<FileChange> {
        let mut changes = Vec::new();
        let now = Instant::now();

        // Drain all pending events
        loop {
            match self.rx.try_recv() {
                Ok(Ok(event)) => {
                    self.process_event(event, &mut changes, now);
                }
                Ok(Err(e)) => {
                    eprintln!("File watcher error: {}", e);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    eprintln!("File watcher channel disconnected");
                    break;
                }
            }
        }

        // Apply debouncing
        changes.retain(|change| {
            let mut last_events = self.last_events.lock().unwrap();
            
            // Check if we've seen this file recently
            if let Some(&last_time) = last_events.get(&change.path) {
                if now.duration_since(last_time) < self.debounce_duration {
                    return false; // Skip this event due to debouncing
                }
            }
            
            // Update last event time
            last_events.insert(change.path.clone(), now);
            true
        });

        changes
    }

    fn process_event(&self, event: Event, changes: &mut Vec<FileChange>, timestamp: Instant) {
        match event.kind {
            EventKind::Create(_) => {
                for path in event.paths {
                    if self.is_jsonl_file(&path) {
                        changes.push(FileChange {
                            path,
                            kind: FileChangeKind::Created,
                            timestamp,
                        });
                    }
                }
            }
            EventKind::Modify(_) => {
                for path in event.paths {
                    if self.is_jsonl_file(&path) {
                        changes.push(FileChange {
                            path,
                            kind: FileChangeKind::Modified,
                            timestamp,
                        });
                    }
                }
            }
            EventKind::Remove(_) => {
                for path in event.paths {
                    if self.is_jsonl_file(&path) {
                        changes.push(FileChange {
                            path,
                            kind: FileChangeKind::Removed,
                            timestamp,
                        });
                    }
                }
            }
            _ => {} // Ignore other event types
        }
    }

    fn is_jsonl_file(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "jsonl")
            .unwrap_or(false)
    }

    /// Add a new path to watch
    pub fn watch_path(&mut self, path: PathBuf) -> Result<()> {
        self._watcher.watch(&path, RecursiveMode::Recursive)?;
        self.watched_paths.lock().unwrap().insert(path);
        Ok(())
    }

    /// Remove a path from watching
    pub fn unwatch_path(&mut self, path: &Path) -> Result<()> {
        self._watcher.unwatch(path)?;
        self.watched_paths.lock().unwrap().remove(path);
        Ok(())
    }

    /// Get the list of currently watched paths
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        self.watched_paths.lock().unwrap().iter().cloned().collect()
    }
}

use std::collections::HashMap;

/// A cross-platform file watcher with fallback support
pub struct CrossPlatformWatcher {
    #[cfg(target_os = "linux")]
    inner: FileWatcher,
    #[cfg(target_os = "macos")]
    inner: FileWatcher,
    #[cfg(target_os = "windows")]
    inner: FileWatcher,
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    inner: PollingWatcher,
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
struct PollingWatcher {
    paths: Vec<PathBuf>,
    last_check: HashMap<PathBuf, SystemTime>,
}

impl CrossPlatformWatcher {
    pub fn new(paths: Vec<PathBuf>) -> Result<Self> {
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
        {
            Ok(Self {
                inner: FileWatcher::new(paths, Duration::from_secs(1))?,
            })
        }
        
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            Ok(Self {
                inner: PollingWatcher {
                    paths,
                    last_check: HashMap::new(),
                },
            })
        }
    }

    pub fn poll_changes(&mut self) -> Vec<FileChange> {
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
        {
            self.inner.poll_changes()
        }
        
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            // Fallback polling implementation
            let mut changes = Vec::new();
            let now = Instant::now();
            
            for path in &self.inner.paths {
                if let Ok(entries) = std::fs::read_dir(path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                            if let Ok(metadata) = entry.metadata() {
                                if let Ok(modified) = metadata.modified() {
                                    let should_report = match self.inner.last_check.get(&path) {
                                        Some(&last) => modified > last,
                                        None => true,
                                    };
                                    
                                    if should_report {
                                        self.inner.last_check.insert(path.clone(), modified);
                                        changes.push(FileChange {
                                            path,
                                            kind: FileChangeKind::Modified,
                                            timestamp: now,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            changes
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::thread;
    use tempfile::TempDir;

    #[test]
    fn test_file_watcher_detects_changes() {
        let temp_dir = TempDir::new().unwrap();
        let watch_path = temp_dir.path().to_path_buf();
        
        // Create watcher
        let watcher = FileWatcher::new(vec![watch_path.clone()], Duration::from_millis(100))
            .expect("Failed to create watcher");
        
        // Create a JSONL file
        let file_path = watch_path.join("test.jsonl");
        fs::write(&file_path, "initial content").unwrap();
        
        // Give the watcher time to detect the creation
        thread::sleep(Duration::from_millis(200));
        
        // Poll for changes
        let changes = watcher.poll_changes();
        assert!(!changes.is_empty(), "Should detect file creation");
        
        // Verify the change
        let change = &changes[0];
        assert_eq!(change.path, file_path);
        assert_eq!(change.kind, FileChangeKind::Created);
    }

    #[test]
    fn test_file_watcher_ignores_non_jsonl() {
        let temp_dir = TempDir::new().unwrap();
        let watch_path = temp_dir.path().to_path_buf();
        
        let watcher = FileWatcher::new(vec![watch_path.clone()], Duration::from_millis(100))
            .expect("Failed to create watcher");
        
        // Create non-JSONL files
        fs::write(watch_path.join("test.txt"), "content").unwrap();
        fs::write(watch_path.join("test.json"), "{}").unwrap();
        
        thread::sleep(Duration::from_millis(200));
        
        let changes = watcher.poll_changes();
        assert!(changes.is_empty(), "Should ignore non-JSONL files");
    }

    #[test]
    fn test_debouncing() {
        let temp_dir = TempDir::new().unwrap();
        let watch_path = temp_dir.path().to_path_buf();
        
        let watcher = FileWatcher::new(vec![watch_path.clone()], Duration::from_millis(500))
            .expect("Failed to create watcher");
        
        let file_path = watch_path.join("test.jsonl");
        
        // Rapid file modifications
        for i in 0..5 {
            fs::write(&file_path, format!("content {}", i)).unwrap();
            thread::sleep(Duration::from_millis(50));
        }
        
        // First poll should get the change
        let changes1 = watcher.poll_changes();
        assert!(!changes1.is_empty());
        
        // Immediate second poll should be empty due to debouncing
        let changes2 = watcher.poll_changes();
        assert!(changes2.is_empty());
        
        // After debounce period, should see changes again if file was modified
        thread::sleep(Duration::from_millis(600));
        fs::write(&file_path, "new content").unwrap();
        thread::sleep(Duration::from_millis(100));
        
        let changes3 = watcher.poll_changes();
        assert!(!changes3.is_empty());
    }
}