# Claude Usage Tool Development Notes

## Important: Format Change After June 4
- Anthropic changed the message format after June 4
- Only analyze messages after this date
- The data is stored in JSONL files in ~/.claude directory

## Data Location
- JSONL files are located in: `~/.claude/projects/*/`
- Each project directory contains JSONL files with session UUIDs as names
- Each line in JSONL files contains a complete JSON object

## JSON Structure Analysis (Post-June 4 Format)

### Important: Multiple Entry Types Exist

There are at least 3 different entry types in the JSONL files:

1. **Summary Entries** (type: "summary"):
```json
{
  "type": "summary",
  "summary": "Brief description of conversation",
  "leafUuid": "uuid-string"
}
```

2. **Older Format Messages** (missing message.id):
```json
{
  "type": "user" | "assistant",
  "uuid": "unique identifier",
  "message": {
    "role": "user" | "assistant",
    "content": [...],
    // Note: No "id" field in older format
  }
}
```

3. **Current Format Messages** (complete structure):
```json
{
  "type": "user" | "assistant",
  "uuid": "unique identifier",
  "message": {
    "id": "msg_...",  // This field is missing in older messages
    "role": "user" | "assistant",
    "model": "claude-...",
    "usage": { ... }
  }
}
```

### Key Fields in Each Entry:
```json
{
  "type": "user" | "assistant" | "summary",
  "uuid": "unique identifier for this entry",
  "parentUuid": "links to previous entry in conversation",
  "timestamp": "ISO 8601 format (e.g., 2025-06-12T14:41:33.572Z)",
  "sessionId": "identifies the conversation session",
  "requestId": "unique ID for API requests (used for deduplication)",
  "version": "CLI version (e.g., 1.0.18)",
  "message": {
    "id": "message ID (e.g., msg_01UJ9k1XdEFtTJjpMhBdnSRM)",
    "role": "user" | "assistant",
    "model": "model name (e.g., claude-opus-4-20250514)",
    "usage": {
      "input_tokens": number,
      "output_tokens": number,
      "cache_creation_input_tokens": number,
      "cache_read_input_tokens": number,
      "service_tier": "standard"
    }
  }
}
```

### Important Observations:

1. **Message Deduplication**: 
   - Same `requestId` appears multiple times as conversation progresses
   - Use `requestId` + latest `timestamp` for deduplication
   - Some messages have `requestId: null` (e.g., synthetic messages)

2. **Token Counting Fields**:
   - `input_tokens`: Direct input tokens
   - `output_tokens`: Generated tokens
   - `cache_creation_input_tokens`: Tokens used to create cache
   - `cache_read_input_tokens`: Tokens read from cache

3. **Conversation Structure**:
   - Each entry has a `parentUuid` linking to previous entry
   - `sessionId` groups entries in same conversation
   - `isSidechain` field indicates resumed conversations

4. **Special Message Types**:
   - `type: "summary"` entries exist (conversation summaries)
   - Model `<synthetic>` appears for some system messages
   - Tool use results stored in separate entries

## Known Issues and Solutions

### Malformed Entry Warnings

The tool will show warnings like:
```
Skipping malformed entry in file.jsonl line X: missing field `uuid` at line 1 column Y
Skipping malformed entry in file.jsonl line X: missing field `id` at line 1 column Y
```

These are expected for:
1. **Summary entries** - Don't have `uuid` field (they have `leafUuid` instead)
2. **Older format messages** - Don't have `message.id` field
3. **Continued conversations** - May have different structure

**Current behavior**: The parser skips these entries and continues processing valid entries. This is intentional to maintain compatibility with different Claude CLI versions.

## Development Progress Log
- 2025-06-12 11:55: Initial repository structure created
- 2025-06-12 11:55: Analyzed JSONL format - identified key fields for token counting and deduplication
- 2025-06-12 12:10: Successfully implemented claude-usage tool with all features:
  - JSONL parsing with deduplication by requestId
  - Token counting (input, output, cache write, cache read)
  - Cost calculation using hardcoded Anthropic pricing
  - Multiple grouping options (day, week, month, model)
  - Multiple output formats (table, csv, json, markdown)
  - Date range filtering and model filtering
  - Summary statistics display
- 2025-06-12 12:10: First successful run processed 16,242 requests totaling $3,642.61
- 2025-01-06 17:10: Documented multiple message formats found in JSONL files:
  - Summary entries (type: "summary") without uuid/message fields
  - Older format messages missing message.id field
  - Current format with complete structure
- 2025-01-06 17:15: Updated parser to handle different message types gracefully:
  - Silently skip summary entries (no usage data)
  - Suppress warnings for known missing fields (id, uuid)
  - Only warn about truly unexpected formats

## TODO: Type Safety Improvements

### Use Strong Types Instead of Strings
- Replace `String` for model names with an enum or newtype wrapper
- Example:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Hash)]
  enum ModelName {
      Opus4,
      Sonnet4,
      Haiku3,
      Sonnet3_5,
      Unknown(String), // For forward compatibility
  }
  ```
- Benefits:
  - Compile-time checking for model names
  - Prevents typos and mismatches
  - Easier refactoring
  - Better IDE support
- Areas to update:
  - `LogEntry` struct
  - `Message` struct
  - `ModelPricing` keys
  - Model filter in CLI args and dashboard

## TODO: Implement fetch_latest_pricing

### Complete the Anthropic API Integration
- Currently `fetch_latest_pricing()` just returns hardcoded values
- Need to implement actual API call to fetch current pricing
- Considerations:
  - Anthropic may not have a public pricing API
  - Could scrape from documentation page
  - Could use a community-maintained pricing feed
  - Cache pricing data locally with expiration
- Implementation approach:
  ```rust
  // Option 1: Official API (if available)
  let response = reqwest::get("https://api.anthropic.com/v1/pricing").await?;
  
  // Option 2: Scrape docs page
  let html = reqwest::get("https://docs.anthropic.com/en/docs/about-claude/models").await?;
  // Parse HTML for pricing table
  
  // Option 3: Community API
  let response = reqwest::get("https://community-api.com/anthropic-pricing").await?;
  ```
- Add caching to avoid hitting API too frequently
- Fallback to hardcoded values if API fails