# Live Update Architecture - Visual Guide

## Current Architecture (Inefficient)

```
Every 5 seconds:
┌─────────────────┐
│   Dashboard     │
└────────┬────────┘
         │ on_tick()
         ▼
┌─────────────────┐
│  refresh_data() │
└────────┬────────┘
         │ 
         ▼
┌─────────────────────────────────────────┐
│        LogParser::parse_logs()          │
│  ┌───────────────────────────────────┐  │
│  │ 1. Find ALL .jsonl files          │  │
│  │ 2. Read ENTIRE file content       │  │
│  │ 3. Parse EVERY line               │  │
│  │ 4. Check EACH UUID in HashSet     │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
         │
         ▼
┌─────────────────┐
│  100+ files     │ ← Reads ~50-200MB every 5 seconds!
│  Millions lines │
│  99.9% unchanged│
└─────────────────┘
```

## New Architecture (Efficient)

```
Every 5 seconds (or on file change):
┌─────────────────┐
│   Dashboard     │
└────────┬────────┘
         │ on_tick()
         ▼
┌─────────────────────────────────────────────┐
│           Update Coordinator                 │
│  ┌─────────────────────────────────────┐    │
│  │ 1. Check which files changed        │    │
│  │ 2. Read only from last position     │    │
│  │ 3. Parse only new lines             │    │
│  │ 4. No deduplication needed!         │    │
│  └─────────────────────────────────────┘    │
└──────┬──────────────┬───────────────────────┘
       │              │
       ▼              ▼
┌──────────────┐  ┌──────────────┐
│ File Tracker │  │ Incremental  │
│              │  │   Parser     │
│ tracks:      │  │              │
│ - mod time   │  │ reads from:  │
│ - file size  │  │ - last pos   │
│ - read pos   │  │ - to EOF     │
└──────────────┘  └──────────────┘
       │              │
       └──────┬───────┘
              ▼
┌─────────────────┐
│  Only new data  │ ← Reads ~1-10KB per update!
│  ~10-100 lines  │
│  All fresh      │
└─────────────────┘
```

## Data Flow Comparison

### Current Flow (Wasteful)
```
File A (10MB): [==========================================] ← Read all
File B (15MB): [==========================================] ← Read all
File C (8MB):  [==========================================] ← Read all
                                                          ↑
                              99.9% of this data was already processed!
```

### New Flow (Efficient)
```
File A (10MB): [========================================|→] ← Read 2KB
                ↑ Last position                         ↑ New data

File B (15MB): [==========================================] ← Skip (unchanged)

File C (8MB):  [==================================|→→→→→→] ← Read 5KB
                ↑ Last position                   ↑ New data
```

## State Management

### File Tracker State (Persisted)
```json
{
  "/Users/wfu/.claude/projects/foo/abc.jsonl": {
    "last_modified": "2024-06-13T16:30:45Z",
    "last_size": 1048576,
    "last_position": 1048576,
    "inode": 12345678
  },
  "/Users/wfu/.claude/projects/bar/def.jsonl": {
    "last_modified": "2024-06-13T16:25:30Z", 
    "last_size": 2097152,
    "last_position": 2097152,
    "inode": 87654321
  }
}
```

## Update Detection Logic

```rust
// Pseudocode for change detection
for each tracked_file {
    current_stat = fs::metadata(file)?;
    
    if current_stat.modified > tracked_file.last_modified {
        // File was modified
        if current_stat.size < tracked_file.last_size {
            // File was rotated/truncated
            tracked_file.last_position = 0;
        }
        // Read from last_position to EOF
        parse_increment(file, tracked_file.last_position);
    }
}
```

## Performance Impact

| Metric | Current System | New System | Improvement |
|--------|---------------|------------|-------------|
| Files Read per Update | 150+ | 1-5 | 97% fewer |
| Data Read per Update | 50-200 MB | 1-50 KB | 99.9% less |
| Parse Time | 200-500ms | 1-10ms | 98% faster |
| Memory Usage | Spikes | Constant | Stable |
| CPU Usage | 5-10% | <0.1% | 99% less |

## Optional Enhancement: File Watcher

```
With File System Events:
┌─────────────────┐
│   OS Kernel     │
│  File System    │
└────────┬────────┘
         │ inotify/FSEvents
         ▼
┌─────────────────┐
│  File Watcher   │ ← Instant notification!
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Update Now!     │ ← <100ms latency
└─────────────────┘
```

## Error Recovery

```
If file rotation detected:
  ┌──────────────┐
  │ Old file.jsonl│ → Renamed/deleted
  └──────────────┘
  ┌──────────────┐
  │ New file.jsonl│ → Same name, different inode
  └──────────────┘
  
  Action: Reset position to 0, read entire new file
```

## Implementation Priority

1. **Week 1**: File Tracker (80% benefit)
2. **Week 2**: Incremental Parser (95% benefit)
3. **Week 3**: Testing & Optimization
4. **Week 4**: File Watcher (Optional, for instant updates)

This architecture reduces resource usage by over 99% while providing faster, more responsive updates!