# Progress Log

## 2025-01-06 16:57

- Fixed VSCode Insiders Shift+Enter keybinding for Claude Code multi-line input
  - Located correct keybindings file at `/Users/wfu/Library/Application Support/Code - Insiders/User/keybindings.json`
  - Added keybinding to send `\\\n` when terminalFocus is true
  
- Created GitHub repository and pushed claude-usage tool
  - Created public repository at https://github.com/wfus/cli-tools
  - Fixed unused import warning in formatters.rs
  - Added detailed PATH installation instructions to README
  - Successfully pushed clean code (no warnings) to GitHub
  - Repository structure supports multiple CLI tools for future expansion

## 2025-01-06 17:18

- Investigated and fixed malformed entry warnings in claude-usage
  - Discovered multiple message formats in JSONL files:
    - Summary entries (type: "summary") with different structure
    - Older format messages missing message.id field
    - Current format with complete structure
  - Updated parser to handle different formats gracefully:
    - Silently skip summary entries (they don't contain usage data)
    - Suppress warnings for known missing fields (id, uuid)
    - Only warn about truly unexpected formats
  - Documented all findings in NOTES.md for future reference
  - Result: Cleaner output with less noise while maintaining compatibility

## 2025-01-06 18:00

- Fixed daily/weekly/monthly aggregation to properly sum costs across all models
  - When grouping by day/week/month, now shows "all" as model and aggregates correctly
  - Calculates cost per-model when aggregating to ensure accurate pricing
  
- Created detailed specification for terminal UI dashboard
  - Real-time monitoring with rolling 60-minute window
  - Per-minute cost buckets for granular tracking
  - Model filtering (All/Opus/Sonnet/Haiku) with 'm' key
  - Live request feed showing individual API calls
  - Multiple time ranges (1h/2h/6h/12h/24h)
  - Uses ratatui for cross-platform terminal UI
  - Planned file watcher for real-time JSONL updates