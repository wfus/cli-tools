# Live Update System Design for Claude Usage Dashboard

## Overview

This document describes an efficient live update system for the Claude Usage Dashboard that minimizes disk I/O and CPU usage while providing near real-time updates.

## Current System Problems

1. **Full Re-read Every 5 Seconds**: Re-reads ALL JSONL files from the last 24 hours
2. **Full Re-parse**: Parses every line in every file, even unchanged ones
3. **O(n) Deduplication**: Checks every entry against a growing HashSet
4. **No File Change Detection**: Can't tell which files have new data
5. **Fixed Polling Interval**: Updates every 5 seconds regardless of activity

## Proposed Architecture

### Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                     Dashboard App                            │
├─────────────────────────────────────────────────────────────┤
│                   Update Coordinator                         │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │File Watcher │  │ File Tracker │  │Incremental Parser│  │
│  │  (notify)   │  │  (metadata)  │  │  (tail reader)   │  │
│  └─────────────┘  └──────────────┘  └──────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                    JSONL Files on Disk                       │
└─────────────────────────────────────────────────────────────┘
```

### 1. File Tracker Component

**Purpose**: Track file metadata to detect changes efficiently

**Data Structure**:
```rust
pub struct FileState {
    pub path: PathBuf,
    pub last_modified: SystemTime,
    pub last_size: u64,
    pub last_position: u64,  // Where we last read to
    pub inode: Option<u64>,  // For detecting file rotation
}

pub struct FileTracker {
    states: HashMap<PathBuf, FileState>,
    state_file: PathBuf,  // Persist to disk
}
```

**Key Methods**:
- `track_file(path)` - Start tracking a file
- `check_changes()` - Return list of changed files
- `update_position(path, pos)` - Update read position
- `persist()` / `restore()` - Save/load state

### 2. Incremental Parser Component

**Purpose**: Read only new data from files

**Key Features**:
- Seek to last read position
- Read only new bytes
- Handle file rotation gracefully
- Parse incrementally

```rust
pub struct IncrementalParser {
    file_tracker: FileTracker,
    parser: LogParser,
}

impl IncrementalParser {
    pub fn parse_new_entries(&mut self) -> Result<Vec<LogEntry>> {
        let changed_files = self.file_tracker.check_changes()?;
        let mut new_entries = Vec::new();
        
        for file_info in changed_files {
            let entries = self.parse_file_increment(&file_info)?;
            new_entries.extend(entries);
        }
        
        new_entries.sort_by_key(|e| e.timestamp);
        Ok(new_entries)
    }
    
    fn parse_file_increment(&mut self, file_info: &FileChange) -> Result<Vec<LogEntry>> {
        let mut file = File::open(&file_info.path)?;
        
        // Seek to last read position
        file.seek(SeekFrom::Start(file_info.last_position))?;
        
        // Read only new content
        let mut reader = BufReader::new(file);
        let mut entries = Vec::new();
        let mut current_pos = file_info.last_position;
        
        for line in reader.lines() {
            let line = line?;
            current_pos += line.len() as u64 + 1; // +1 for newline
            
            if let Ok(entry) = serde_json::from_str::<LogEntry>(&line) {
                entries.push(entry);
            }
        }
        
        // Update tracked position
        self.file_tracker.update_position(&file_info.path, current_pos)?;
        
        Ok(entries)
    }
}
```

### 3. File Watcher Component (Optional Enhancement)

**Purpose**: Get instant notifications when files change

**Implementation**:
```rust
use notify::{Watcher, RecursiveMode, Event};

pub struct FileWatcher {
    watcher: RecommendedWatcher,
    rx: Receiver<notify::Result<Event>>,
}

impl FileWatcher {
    pub fn watch_directory(&mut self, path: &Path) -> Result<()> {
        self.watcher.watch(path, RecursiveMode::Recursive)?;
        Ok(())
    }
    
    pub fn get_changes(&self, timeout: Duration) -> Vec<PathBuf> {
        let mut changes = Vec::new();
        
        while let Ok(Ok(event)) = self.rx.recv_timeout(timeout) {
            if matches!(event.kind, EventKind::Modify(_)) {
                changes.extend(event.paths);
            }
        }
        
        changes
    }
}
```

### 4. Update Coordinator

**Purpose**: Orchestrate the update process

```rust
pub struct UpdateCoordinator {
    incremental_parser: IncrementalParser,
    file_watcher: Option<FileWatcher>,
    last_full_scan: Instant,
}

impl UpdateCoordinator {
    pub async fn check_for_updates(&mut self) -> Result<Vec<LogEntry>> {
        // 1. Check file watcher for instant updates
        if let Some(ref watcher) = self.file_watcher {
            let changed = watcher.get_changes(Duration::from_millis(100));
            if !changed.is_empty() {
                return self.incremental_parser.parse_new_entries();
            }
        }
        
        // 2. Periodic scan (every 5 seconds as fallback)
        if self.last_full_scan.elapsed() > Duration::from_secs(5) {
            self.last_full_scan = Instant::now();
            return self.incremental_parser.parse_new_entries();
        }
        
        Ok(vec![])
    }
}
```

## Integration with Dashboard

### Modified App Structure

```rust
pub struct App {
    // ... existing fields ...
    update_coordinator: UpdateCoordinator,
    // Remove seen_request_ids - no longer needed!
}

impl App {
    pub async fn refresh_data(&mut self) -> Result<()> {
        // Much simpler now!
        let new_entries = self.update_coordinator.check_for_updates().await?;
        
        for entry in new_entries {
            if let Some(request) = self.convert_to_request(entry) {
                self.rolling_window.add_request(request.clone());
                
                if !self.feed_paused {
                    self.request_feed.push_front(request);
                    if self.request_feed.len() > 100 {
                        self.request_feed.pop_back();
                    }
                }
            }
        }
        
        self.last_update = Utc::now();
        Ok(())
    }
}
```

## Implementation Phases

### Phase 1: File Tracker (Biggest Impact)
- Implement FileTracker to detect changed files
- Read full files but only changed ones
- 80% reduction in I/O

### Phase 2: Incremental Parsing
- Implement seek-based incremental reading
- Parse only new lines
- 95%+ reduction in I/O

### Phase 3: File Watcher (Optional)
- Add real-time file system monitoring
- Sub-second update latency
- Falls back to polling if not supported

### Phase 4: Optimizations
- Persistent read position cache
- Concurrent file parsing
- Memory-mapped files for large logs

## Benefits

1. **Performance**:
   - Initial load: Same as current
   - Subsequent updates: 95%+ faster
   - CPU usage: Minimal between updates

2. **Responsiveness**:
   - With file watcher: <100ms latency
   - Without: Still 5-second polling

3. **Scalability**:
   - Handles thousands of JSONL files
   - Grows with O(new data) not O(total data)

4. **Reliability**:
   - Handles file rotation
   - Persists state across restarts
   - Graceful fallbacks

## Migration Strategy

1. **Keep Current System**: Leave UUID deduplication as fallback
2. **Add New System**: Run in parallel initially
3. **Verify Correctness**: Compare outputs
4. **Switch Over**: Use new system primarily
5. **Remove Old Code**: After proven stable

## Error Handling

- **File Rotation**: Detect via inode/size changes
- **Corrupted State**: Fall back to full scan
- **Missing Files**: Remove from tracker
- **Parse Errors**: Skip bad lines, continue

## Testing Strategy

1. **Unit Tests**:
   - File tracker state management
   - Incremental parsing logic
   - File rotation detection

2. **Integration Tests**:
   - Multi-file scenarios
   - Concurrent modifications
   - Large file handling

3. **Stress Tests**:
   - Rapid file updates
   - Many simultaneous files
   - Large backlogs

## Performance Metrics

Track and display:
- Files checked per refresh
- Bytes read per refresh
- Entries parsed per refresh
- Time spent in refresh
- Cache hit rate

## Future Enhancements

1. **Compressed Logs**: Support .jsonl.gz
2. **Remote Logs**: S3/GCS support
3. **Query Pushdown**: Filter at read time
4. **Bloom Filters**: Faster duplicate detection
5. **Index Files**: Binary search in large files

## Conclusion

This design provides a robust, efficient live update system that scales well and provides near real-time updates with minimal resource usage. The phased implementation allows for incremental improvements while maintaining system stability.