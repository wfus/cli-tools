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