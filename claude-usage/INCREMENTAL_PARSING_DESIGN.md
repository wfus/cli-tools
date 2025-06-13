# Incremental Parsing Design for Claude Usage Dashboard

## Overview
This document outlines a more efficient approach for fetching new data in the Claude Usage dashboard. The current implementation re-reads all JSONL files every 5 seconds, which is inefficient and causes unnecessary disk I/O.

## Current Issues
1. **Full file re-parsing**: Every 5 seconds, all JSONL files are read from the beginning
2. **No change detection**: Files are read even if they haven't been modified
3. **Redundant parsing**: Previously seen entries are parsed again
4. **High disk I/O**: Continuous reading of potentially large files

## Proposed Solution

### 1. File State Tracker
Create a new module `src/file_tracker.rs` that maintains state about each JSONL file:

```rust
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileState {
    pub path: PathBuf,
    pub last_modified: SystemTime,
    pub last_read_position: u64,
    pub last_line_number: usize,
    pub file_size: u64,
}

pub struct FileTracker {
    // Map from file path to its state
    states: HashMap<PathBuf, FileState>,
    // Optional: persist state to disk for resilience
    state_file: Option<PathBuf>,
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
        tracker.load_state();
        tracker
    }
    
    pub fn check_file(&mut self, path: &Path) -> FileCheckResult {
        let metadata = fs::metadata(path)?;
        let current_modified = metadata.modified()?;
        let current_size = metadata.len();
        
        match self.states.get(path) {
            Some(state) => {
                if state.last_modified < current_modified || state.file_size < current_size {
                    FileCheckResult::Modified {
                        last_position: state.last_read_position,
                        last_line: state.last_line_number,
                    }
                } else {
                    FileCheckResult::Unchanged
                }
            }
            None => FileCheckResult::New
        }
    }
    
    pub fn update_state(&mut self, path: PathBuf, position: u64, line_number: usize) {
        let metadata = fs::metadata(&path).unwrap();
        self.states.insert(path.clone(), FileState {
            path,
            last_modified: metadata.modified().unwrap(),
            last_read_position: position,
            last_line_number: line_number,
            file_size: metadata.len(),
        });
        self.save_state();
    }
}

pub enum FileCheckResult {
    New,
    Modified { last_position: u64, last_line: usize },
    Unchanged,
}
```

### 2. Incremental Parser
Modify the `LogParser` to support incremental parsing:

```rust
impl LogParser {
    pub fn parse_logs_incremental(&self, tracker: &mut FileTracker) -> Result<Vec<LogEntry>> {
        let expanded_path = shellexpand::tilde(&self.claude_dir).to_string();
        let projects_dir = Path::new(&expanded_path).join("projects");
        
        let jsonl_files = self.find_jsonl_files(&projects_dir)?;
        let mut all_entries = Vec::new();
        
        for file_path in jsonl_files {
            match tracker.check_file(&file_path) {
                FileCheckResult::Unchanged => continue,
                FileCheckResult::New => {
                    let entries = self.parse_jsonl_file(&file_path)?;
                    tracker.update_state(file_path, /* position */, /* line_number */);
                    all_entries.extend(entries);
                }
                FileCheckResult::Modified { last_position, last_line } => {
                    let entries = self.parse_jsonl_file_from_position(
                        &file_path, 
                        last_position,
                        last_line
                    )?;
                    tracker.update_state(file_path, /* new_position */, /* new_line */);
                    all_entries.extend(entries);
                }
            }
        }
        
        Ok(self.deduplicate_entries(all_entries))
    }
    
    fn parse_jsonl_file_from_position(
        &self, 
        path: &Path, 
        start_position: u64,
        start_line: usize
    ) -> Result<(Vec<LogEntry>, u64, usize)> {
        let mut file = File::open(path)?;
        file.seek(SeekFrom::Start(start_position))?;
        
        let mut reader = BufReader::new(file);
        let mut entries = Vec::new();
        let mut line_num = start_line;
        let mut last_position = start_position;
        
        for line in reader.lines() {
            let line = line?;
            line_num += 1;
            
            if line.trim().is_empty() {
                continue;
            }
            
            // Parse line as before...
            // Track position after each line
            last_position = file.stream_position()?;
        }
        
        Ok((entries, last_position, line_num))
    }
}
```

### 3. File System Watcher (Optional Enhancement)
For real-time updates, implement file system watching:

```rust
use notify::{Watcher, RecursiveMode, watcher, DebouncedEvent};
use std::sync::mpsc::channel;
use std::time::Duration;

pub struct FileWatcher {
    watcher: notify::RecommendedWatcher,
    rx: std::sync::mpsc::Receiver<DebouncedEvent>,
}

impl FileWatcher {
    pub fn new(paths: Vec<PathBuf>) -> Result<Self> {
        let (tx, rx) = channel();
        let mut watcher = watcher(tx, Duration::from_secs(1))?;
        
        for path in paths {
            watcher.watch(path, RecursiveMode::Recursive)?;
        }
        
        Ok(Self { watcher, rx })
    }
    
    pub fn check_events(&self) -> Vec<PathBuf> {
        let mut modified_files = Vec::new();
        
        while let Ok(event) = self.rx.try_recv() {
            match event {
                DebouncedEvent::Write(path) | 
                DebouncedEvent::Create(path) => {
                    if path.extension().map_or(false, |ext| ext == "jsonl") {
                        modified_files.push(path);
                    }
                }
                _ => {}
            }
        }
        
        modified_files
    }
}
```

### 4. Integration with Dashboard App
Update the `App` struct and refresh logic:

```rust
pub struct App {
    // ... existing fields ...
    file_tracker: FileTracker,
    #[cfg(feature = "file-watch")]
    file_watcher: Option<FileWatcher>,
}

impl App {
    pub fn new(claude_dir: String, initial_hours: usize) -> Self {
        let state_file = Path::new(&claude_dir).join(".claude-usage-state.json");
        let file_tracker = FileTracker::with_persistence(state_file);
        
        #[cfg(feature = "file-watch")]
        let file_watcher = {
            let projects_dir = Path::new(&claude_dir).join("projects");
            FileWatcher::new(vec![projects_dir]).ok()
        };
        
        Self {
            // ... existing fields ...
            file_tracker,
            #[cfg(feature = "file-watch")]
            file_watcher,
        }
    }
    
    pub async fn refresh_data(&mut self) -> Result<()> {
        // Check for file system events if watcher is enabled
        #[cfg(feature = "file-watch")]
        if let Some(ref watcher) = self.file_watcher {
            let modified_files = watcher.check_events();
            if !modified_files.is_empty() {
                // Trigger immediate refresh for modified files
                self.file_tracker.mark_files_modified(modified_files);
            }
        }
        
        // Use incremental parser
        let parser = LogParser::new(self.claude_dir.clone())
            .with_date_range(Some(self.get_cutoff_date()), None)
            .quiet();
        
        let entries = parser.parse_logs_incremental(&mut self.file_tracker)?;
        
        // Process only new entries
        for entry in entries {
            if self.seen_request_ids.contains(&entry.uuid) {
                continue;
            }
            // ... existing processing logic ...
        }
        
        Ok(())
    }
}
```

## Implementation Steps

1. **Phase 1: File State Tracking**
   - Implement `FileTracker` struct
   - Add file modification time checking
   - Store last read position for each file

2. **Phase 2: Incremental Parsing**
   - Modify `LogParser` to support reading from position
   - Implement `parse_jsonl_file_from_position`
   - Update file state after successful parsing

3. **Phase 3: Optimization**
   - Add state persistence to survive restarts
   - Implement file rotation detection
   - Add error recovery for corrupted state

4. **Phase 4: Real-time Updates (Optional)**
   - Add `notify` crate dependency
   - Implement file system watcher
   - Make it feature-flagged for cross-platform compatibility

## Benefits

1. **Reduced I/O**: Only read changed portions of files
2. **Faster Updates**: Skip unchanged files entirely
3. **Lower CPU Usage**: Parse only new entries
4. **Scalability**: Handles growing log files efficiently
5. **Resilience**: State persistence survives crashes

## Cross-Platform Considerations

1. **File Modification Times**: Use `std::fs::Metadata::modified()` which works on all platforms
2. **File Watching**: Use `notify` crate with fallback to polling on unsupported platforms
3. **Path Handling**: Use `std::path::Path` for proper path separators
4. **State File Location**: Store in platform-appropriate location

## Error Handling

1. **File Rotation**: Detect when file size decreases (indicates rotation)
2. **Corrupted State**: Fallback to full parse if state is invalid
3. **Missing Files**: Remove from tracker if files are deleted
4. **Permission Errors**: Gracefully skip inaccessible files

## Testing Strategy

1. **Unit Tests**: Test file state tracking logic
2. **Integration Tests**: Test incremental parsing with mock files
3. **Stress Tests**: Verify performance with large files
4. **Edge Cases**: Test file rotation, corruption, concurrent writes

## Migration Path

1. Start with file state tracking (backward compatible)
2. Roll out incremental parsing gradually
3. Add file watching as opt-in feature
4. Monitor performance improvements
5. Remove old parsing code after validation