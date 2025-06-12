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