use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileState {
    pub path: PathBuf,
    pub last_modified: SystemTime,
    pub last_read_position: u64,
    pub last_line_number: usize,
    pub file_size: u64,
    pub inode: Option<u64>, // For detecting file rotation on Unix
}

#[derive(Debug)]
pub enum FileCheckResult {
    /// File is new and hasn't been tracked before
    New,
    /// File has been modified since last check
    Modified {
        last_position: u64,
        last_line: usize,
    },
    /// File hasn't changed
    Unchanged,
    /// File was rotated (size decreased or inode changed)
    Rotated,
}

pub struct FileTracker {
    states: HashMap<PathBuf, FileState>,
    state_file: Option<PathBuf>,
}

impl Default for FileTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl FileTracker {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
            state_file: None,
        }
    }

    pub fn with_persistence(state_file: PathBuf) -> Self {
        let mut tracker = Self {
            states: HashMap::new(),
            state_file: Some(state_file.clone()),
        };
        if let Err(e) = tracker.load_state() {
            eprintln!("Warning: Failed to load file tracker state: {}", e);
        }
        tracker
    }

    pub fn check_file(&self, path: &Path) -> Result<FileCheckResult> {
        let metadata = fs::metadata(path)
            .with_context(|| format!("Failed to get metadata for {}", path.display()))?;
        let current_modified = metadata.modified()?;
        let current_size = metadata.len();

        #[cfg(unix)]
        let current_inode = {
            use std::os::unix::fs::MetadataExt;
            Some(metadata.ino())
        };
        #[cfg(not(unix))]
        let current_inode = None;

        match self.states.get(path) {
            Some(state) => {
                // Check for file rotation
                if current_size < state.file_size {
                    return Ok(FileCheckResult::Rotated);
                }

                #[cfg(unix)]
                if let (Some(old_inode), Some(new_inode)) = (state.inode, current_inode) {
                    if old_inode != new_inode {
                        return Ok(FileCheckResult::Rotated);
                    }
                }

                // Check if file was modified
                if state.last_modified < current_modified || state.file_size < current_size {
                    Ok(FileCheckResult::Modified {
                        last_position: state.last_read_position,
                        last_line: state.last_line_number,
                    })
                } else {
                    Ok(FileCheckResult::Unchanged)
                }
            }
            None => Ok(FileCheckResult::New),
        }
    }

    pub fn update_state(
        &mut self,
        path: PathBuf,
        position: u64,
        line_number: usize,
    ) -> Result<()> {
        let metadata = fs::metadata(&path)?;

        #[cfg(unix)]
        let inode = {
            use std::os::unix::fs::MetadataExt;
            Some(metadata.ino())
        };
        #[cfg(not(unix))]
        let inode = None;

        self.states.insert(
            path.clone(),
            FileState {
                path,
                last_modified: metadata.modified()?,
                last_read_position: position,
                last_line_number: line_number,
                file_size: metadata.len(),
                inode,
            },
        );

        if self.state_file.is_some() {
            self.save_state()?;
        }

        Ok(())
    }

    pub fn remove_file(&mut self, path: &Path) {
        self.states.remove(path);
        if self.state_file.is_some() {
            let _ = self.save_state();
        }
    }

    pub fn clear(&mut self) {
        self.states.clear();
        if self.state_file.is_some() {
            let _ = self.save_state();
        }
    }

    fn load_state(&mut self) -> Result<()> {
        if let Some(ref state_file) = self.state_file {
            if state_file.exists() {
                let file = File::open(state_file)?;
                let reader = BufReader::new(file);
                self.states = serde_json::from_reader(reader)?;
            }
        }
        Ok(())
    }

    fn save_state(&self) -> Result<()> {
        if let Some(ref state_file) = self.state_file {
            // Create parent directory if it doesn't exist
            if let Some(parent) = state_file.parent() {
                fs::create_dir_all(parent)?;
            }

            let file = File::create(state_file)?;
            let writer = BufWriter::new(file);
            serde_json::to_writer_pretty(writer, &self.states)?;
        }
        Ok(())
    }

    /// Get the number of tracked files
    pub fn tracked_files_count(&self) -> usize {
        self.states.len()
    }

    /// Get total bytes read across all tracked files
    pub fn total_bytes_read(&self) -> u64 {
        self.states.values().map(|s| s.last_read_position).sum()
    }

    /// Check if we're tracking a specific file
    pub fn is_tracking(&self, path: &Path) -> bool {
        self.states.contains_key(path)
    }

    /// Mark files as potentially modified (useful for file watcher integration)
    pub fn mark_files_modified(&mut self, paths: Vec<PathBuf>) {
        for path in paths {
            if let Some(state) = self.states.get_mut(&path) {
                // Set modified time to epoch to force re-check
                state.last_modified = SystemTime::UNIX_EPOCH;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_file_tracker_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.jsonl");
        fs::write(&file_path, "test content").unwrap();

        let tracker = FileTracker::new();
        let result = tracker.check_file(&file_path).unwrap();

        match result {
            FileCheckResult::New => (),
            _ => panic!("Expected New, got {:?}", result),
        }
    }

    #[test]
    fn test_file_tracker_modified_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.jsonl");
        fs::write(&file_path, "initial content").unwrap();

        let mut tracker = FileTracker::new();
        tracker.update_state(file_path.clone(), 10, 1).unwrap();

        // Modify the file
        std::thread::sleep(std::time::Duration::from_millis(10));
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&file_path)
            .unwrap();
        writeln!(file, "new line").unwrap();

        let result = tracker.check_file(&file_path).unwrap();
        match result {
            FileCheckResult::Modified {
                last_position,
                last_line,
            } => {
                assert_eq!(last_position, 10);
                assert_eq!(last_line, 1);
            }
            _ => panic!("Expected Modified, got {:?}", result),
        }
    }

    #[test]
    fn test_file_tracker_unchanged_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.jsonl");
        fs::write(&file_path, "test content").unwrap();

        let mut tracker = FileTracker::new();
        let file_size = fs::metadata(&file_path).unwrap().len();
        tracker.update_state(file_path.clone(), file_size, 1).unwrap();

        let result = tracker.check_file(&file_path).unwrap();
        match result {
            FileCheckResult::Unchanged => (),
            _ => panic!("Expected Unchanged, got {:?}", result),
        }
    }

    #[test]
    fn test_file_tracker_rotated_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.jsonl");
        fs::write(&file_path, "initial long content").unwrap();

        let mut tracker = FileTracker::new();
        tracker.update_state(file_path.clone(), 20, 1).unwrap();

        // Simulate file rotation by writing shorter content
        fs::write(&file_path, "short").unwrap();

        let result = tracker.check_file(&file_path).unwrap();
        match result {
            FileCheckResult::Rotated => (),
            _ => panic!("Expected Rotated, got {:?}", result),
        }
    }

    #[test]
    fn test_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let state_file = temp_dir.path().join("state.json");
        let file_path = temp_dir.path().join("test.jsonl");
        fs::write(&file_path, "test content").unwrap();

        // Create tracker and save state
        {
            let mut tracker = FileTracker::with_persistence(state_file.clone());
            tracker.update_state(file_path.clone(), 42, 5).unwrap();
        }

        // Load state in new tracker
        let tracker = FileTracker::with_persistence(state_file);
        assert!(tracker.is_tracking(&file_path));

        let result = tracker.check_file(&file_path).unwrap();
        match result {
            FileCheckResult::Unchanged => (),
            _ => panic!("Expected Unchanged after loading state"),
        }
    }
}