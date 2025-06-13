# Integration Guide for Incremental Parsing

This guide shows how to integrate the incremental parsing components into the Claude Usage dashboard.

## 1. Update Cargo.toml

Add the required dependencies:

```toml
[dependencies]
# ... existing dependencies ...
serde_json = "1.0"
notify = { version = "6.1", optional = true }

[features]
default = []
file-watch = ["notify"]
```

## 2. Update src/lib.rs

Add the new modules:

```rust
pub mod file_tracker;
pub mod incremental_parser;

#[cfg(feature = "file-watch")]
pub mod file_watcher;
```

## 3. Update the App struct (src/dashboard/app.rs)

```rust
use crate::file_tracker::FileTracker;
#[cfg(feature = "file-watch")]
use crate::file_watcher::{CrossPlatformWatcher, FileChangeKind};
use crate::incremental_parser::IncrementalParsing;

pub struct App {
    // ... existing fields ...
    
    // Add these new fields:
    file_tracker: FileTracker,
    #[cfg(feature = "file-watch")]
    file_watcher: Option<CrossPlatformWatcher>,
    last_full_parse: DateTime<Utc>,
    incremental_parse_enabled: bool,
}

impl App {
    pub fn new(claude_dir: String, initial_hours: usize) -> Self {
        // Set up file tracker with persistence
        let state_file = shellexpand::tilde(&claude_dir)
            .into_owned()
            .into_path()
            .join(".claude-usage-tracker.json");
        let file_tracker = FileTracker::with_persistence(state_file);
        
        // Set up file watcher if feature is enabled
        #[cfg(feature = "file-watch")]
        let file_watcher = {
            let projects_dir = shellexpand::tilde(&claude_dir)
                .into_owned()
                .into_path()
                .join("projects");
            CrossPlatformWatcher::new(vec![projects_dir]).ok()
        };
        
        Self {
            // ... existing field initialization ...
            file_tracker,
            #[cfg(feature = "file-watch")]
            file_watcher,
            last_full_parse: Utc::now() - Duration::hours(25), // Force initial full parse
            incremental_parse_enabled: true,
        }
    }
    
    pub async fn refresh_data(&mut self) -> Result<()> {
        // Check if we should do a full parse (every 24 hours for safety)
        let should_full_parse = Utc::now() - self.last_full_parse > Duration::hours(24);
        
        // Check for file system events if watcher is enabled
        #[cfg(feature = "file-watch")]
        if let Some(ref mut watcher) = self.file_watcher {
            let changes = watcher.poll_changes();
            for change in changes {
                match change.kind {
                    FileChangeKind::Removed => {
                        self.file_tracker.remove_file(&change.path);
                    }
                    _ => {
                        // Mark file as modified for next parse
                        self.file_tracker.mark_files_modified(vec![change.path]);
                    }
                }
            }
        }
        
        let parser = LogParser::new(self.claude_dir.clone())
            .with_date_range(Some(Utc::now() - Duration::hours(24)), None)
            .quiet();
        
        let entries = if should_full_parse || !self.incremental_parse_enabled {
            // Do a full parse occasionally or if incremental is disabled
            self.file_tracker.clear(); // Reset tracker for full parse
            self.last_full_parse = Utc::now();
            parser.parse_logs()?
        } else {
            // Use incremental parsing
            parser.parse_logs_incremental(&mut self.file_tracker)?
        };
        
        // Process entries as before...
        let mut new_requests = Vec::new();
        
        for entry in entries {
            if self.seen_request_ids.contains(&entry.uuid) {
                continue;
            }
            
            if let Some(message) = &entry.message {
                if let Some(usage) = &message.usage {
                    if !message.model.is_synthetic() {
                        let request = RequestInfo {
                            timestamp: entry.timestamp,
                            model: message.model.clone(),
                            input_tokens: usage.input_tokens as u32,
                            output_tokens: usage.output_tokens as u32,
                            cache_tokens: (usage.cache_creation_input_tokens 
                                + usage.cache_read_input_tokens) as u32,
                            cost: self.calculate_cost(&message.model, usage),
                        };
                        
                        self.rolling_window.add_request(request.clone());
                        new_requests.push(request);
                        self.seen_request_ids.insert(entry.uuid.clone());
                    }
                }
            }
        }
        
        // Update request feed
        if !self.feed_paused {
            for request in new_requests.into_iter().rev() {
                self.request_feed.push_front(request);
                if self.request_feed.len() > 100 {
                    self.request_feed.pop_back();
                }
            }
        }
        
        self.last_update = Utc::now();
        Ok(())
    }
    
    pub fn toggle_incremental_parsing(&mut self) {
        self.incremental_parse_enabled = !self.incremental_parse_enabled;
        if !self.incremental_parse_enabled {
            // Clear tracker when disabling incremental parsing
            self.file_tracker.clear();
        }
    }
    
    pub fn get_parsing_stats(&self) -> (usize, u64) {
        (
            self.file_tracker.tracked_files_count(),
            self.file_tracker.total_bytes_read(),
        )
    }
}
```

## 4. Add UI Controls (src/dashboard/events.rs)

Add keyboard shortcuts to control incremental parsing:

```rust
pub fn handle_key_event(key: event::KeyEvent, app: &mut App) {
    match key.code {
        // ... existing key handlers ...
        
        KeyCode::Char('i') => {
            app.toggle_incremental_parsing();
        }
        KeyCode::Char('r') => {
            // Force full refresh
            app.file_tracker.clear();
            app.last_full_parse = Utc::now() - Duration::hours(25);
        }
        // ... rest of handlers ...
    }
}
```

## 5. Update UI to Show Stats (src/dashboard/ui.rs)

Add parsing statistics to the UI:

```rust
// In the stats panel or status bar
let (tracked_files, bytes_read) = app.get_parsing_stats();
let parsing_mode = if app.incremental_parse_enabled {
    format!("Incremental ({} files tracked)", tracked_files)
} else {
    "Full".to_string()
};

// Add to the UI
let parsing_info = format!(
    "Parsing: {} | Read: {}",
    parsing_mode,
    format_bytes(bytes_read)
);
```

## 6. Build and Run

```bash
# Build with file watching support
cargo build --release --features file-watch

# Or without file watching (cross-platform)
cargo build --release

# Run the dashboard
./target/release/claude-usage dashboard
```

## Performance Improvements

With this implementation, you should see:

1. **Reduced I/O**: Only modified files are read
2. **Faster Updates**: Skip parsing of unchanged files
3. **Lower CPU**: Parse only new log entries
4. **Better Responsiveness**: Real-time updates with file watching
5. **Resilience**: State persists across restarts

## Monitoring Performance

The dashboard now shows:
- Number of files being tracked
- Total bytes read in current session
- Parsing mode (incremental vs full)

Use the 'i' key to toggle between incremental and full parsing to compare performance.

## Troubleshooting

1. **High Memory Usage**: The file tracker state grows over time. The 24-hour full parse cycle helps reset this.

2. **Missing Updates**: If updates are missed, press 'r' to force a full refresh.

3. **File Rotation Issues**: The system automatically detects file rotations when file size decreases or inode changes.

4. **Performance on Large Datasets**: For very large log directories, consider:
   - Reducing the refresh interval
   - Limiting the date range for parsing
   - Using file watching for immediate updates

## Future Enhancements

1. **Compressed Logs**: Support for .jsonl.gz files
2. **Parallel Parsing**: Parse multiple files concurrently
3. **Streaming Parser**: Process files without loading into memory
4. **Remote Logs**: Support for S3 or network file systems
5. **Metrics Export**: Export parsing performance metrics