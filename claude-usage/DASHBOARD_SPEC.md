# Claude Usage Dashboard Specification

## Overview
A real-time terminal UI dashboard for monitoring Claude API usage, costs, and trends with minute-by-minute granularity.

## Command Structure
```bash
# Launch the dashboard
claude-usage dashboard

# Shorthand
claude-usage dash

# With options
claude-usage dashboard --refresh 5 --hours 2
```

## Technical Stack
- **ratatui** - Terminal UI framework (successor to tui-rs)
- **crossterm** - Cross-platform terminal manipulation
- **tokio** - Async runtime for periodic updates
- Reuse existing parser and models from claude-usage

## Dashboard Layout

```
┌─ Claude Usage Dashboard ─────────────────────────────────────────────────┐
│ Model: All Models ▼ | Last Update: 09:55:23 | Auto-refresh: 5s          │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│ Rolling 60-Minute Usage                          Current Hour Stats      │
│ ┌────────────────────────────────────────┐      ┌─────────────────────┐ │
│ │ $0.50 ┤                                │      │ Requests:        47 │ │
│ │ $0.40 ┤      █                     █  │      │ Tokens:       1.2M  │ │
│ │ $0.30 ┤    █ █ █                 █ █ │      │ Cost:        $4.57  │ │
│ │ $0.20 ┤  █ █ █ █ █             █ █ █ │      │                     │ │
│ │ $0.10 ┤█ █ █ █ █ █ █ █     █ █ █ █ █ │      │ By Model:           │ │
│ │ $0.00 └────────────────────────────────┘      │ ▪ Opus:      $3.82  │ │
│ │       -60  -50  -40  -30  -20  -10  now│      │ ▪ Sonnet:    $0.75  │ │
│ └────────────────────────────────────────┘      │ ▪ Haiku:     $0.00  │ │
│                                                  └─────────────────────┘ │
│ Live Request Feed                                                        │
│ ┌────────────────────────────────────────────────────────────────────┐ │
│ │ [09:55:21] Opus     │ 2,345 in / 1,234 out │ Cache: 567 │ $0.08    │ │
│ │ [09:55:18] Sonnet   │   567 in /   890 out │ Cache:   0 │ $0.02    │ │
│ │ [09:55:12] Opus     │ 1,234 in / 2,345 out │ Cache: 123 │ $0.12    │ │
│ │ [09:54:58] Opus     │ 3,456 in / 1,890 out │ Cache: 234 │ $0.15    │ │
│ │ [09:54:45] Sonnet   │   890 in /   567 out │ Cache:   0 │ $0.01    │ │
│ │ [09:54:32] Opus     │ 2,345 in / 3,456 out │ Cache: 456 │ $0.18    │ │
│ │ [09:54:21] Haiku    │   234 in /   345 out │ Cache:   0 │ $0.001   │ │
│ └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│ 24-Hour Summary                                                          │
│ ┌────────────────────────────────────────────────────────────────────┐ │
│ │ Total: $342.56 │ Opus: $298.42 │ Sonnet: $42.14 │ Haiku: $2.00    │ │
│ └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│ [q]uit [m]odel [t]ime-range [↑↓] scroll [p]ause [h]elp                  │
└──────────────────────────────────────────────────────────────────────────┘
```

## Key Features

### 1. Real-time Minute-by-Minute Updates
- Rolling 60-minute window with per-minute cost buckets
- Auto-refresh every N seconds (configurable)
- Watch for new JSONL entries in real-time
- Smooth scrolling as new minutes appear

### 2. Model Filtering
- **Model Selector**: Dropdown to switch between "All Models" or specific models
- Press 'm' to cycle through: All → Opus → Sonnet → Haiku → All
- Stats and graphs update instantly when switching models
- Model-specific color coding in graphs

### 3. Live Request Feed
- Shows individual API calls as they happen
- Displays: timestamp, model, input/output tokens, cache usage, cost
- Auto-scrolls with new requests (pausable)
- Color-coded by model for quick identification

### 4. Time Range Views
- **Default**: 60-minute rolling window
- Press 't' to cycle: 1h → 2h → 6h → 12h → 24h
- Adjusts bucket size automatically (1min for 1h, 5min for 6h, etc.)
- Maintains minute-level precision for recent data

### 5. Interactive Controls
- `m` - Cycle through model filters
- `t` - Change time range
- `↑↓` - Scroll through request feed
- `p` - Pause/unpause auto-scroll
- `r` - Force refresh
- `h` - Show help overlay
- `q` - Quit

### 6. Configuration Options
```bash
claude-usage dashboard [OPTIONS]

OPTIONS:
    --refresh <SECONDS>     Refresh interval in seconds [default: 5]
    --hours <HOURS>        Initial time range in hours [default: 1]
    --model <MODEL>        Start with specific model filter [default: all]
    --no-feed              Hide live request feed
    --compact              Compact mode for smaller terminals
```

## Implementation Plan

### Phase 1: Basic Dashboard Structure
1. Add `dashboard` subcommand to CLI
2. Set up ratatui with basic layout
3. Create app state structure for real-time data
4. Implement quit and basic navigation

### Phase 2: Real-time Data Pipeline
1. Create minute-bucket aggregator
2. Implement JSONL file watcher for new entries
3. Build rolling window data structure (last N minutes)
4. Set up tokio async refresh loop

### Phase 3: Core Visualizations
1. Implement rolling cost chart (bar chart with minute buckets)
2. Add current hour stats panel
3. Create live request feed with scrolling
4. Add 24-hour summary bar

### Phase 4: Model Filtering & Interactivity
1. Implement model filter state and switching
2. Add time range controls (1h/2h/6h/12h/24h)
3. Connect all controls to data updates
4. Add pause/resume for live feed

### Phase 5: Polish & Performance
1. Add model-specific colors
2. Optimize for high-frequency updates
3. Handle edge cases (empty data, terminal resize)
4. Add help overlay
5. Implement compact mode for small terminals

## Dependencies to Add
```toml
# Cargo.toml additions
ratatui = "0.27"
crossterm = "0.27"
unicode-width = "0.1"
notify = "6.1"  # File system watcher for new JSONL entries
```

## File Structure
```
src/
├── dashboard/
│   ├── mod.rs          # Dashboard module
│   ├── app.rs          # Application state & model filter
│   ├── ui.rs           # Main UI layout and rendering
│   ├── data.rs         # Real-time data aggregation
│   ├── widgets/        # Custom widgets
│   │   ├── mod.rs
│   │   ├── minute_chart.rs   # Rolling minute-bucket chart
│   │   ├── stats_panel.rs    # Current hour statistics
│   │   ├── request_feed.rs   # Live request feed
│   │   └── summary_bar.rs    # 24-hour summary
│   └── events.rs       # Keyboard/timer event handling
├── cli.rs              # Add dashboard subcommand
└── main.rs             # Handle dashboard launch
```

## Key Data Structures

### MinuteBucket
```rust
struct MinuteBucket {
    timestamp: DateTime<Utc>,
    requests: Vec<RequestInfo>,
    total_cost: f64,
    model_costs: HashMap<String, f64>,
}

struct RequestInfo {
    timestamp: DateTime<Utc>,
    model: String,
    input_tokens: u32,
    output_tokens: u32,
    cache_tokens: u32,
    cost: f64,
}
```

### RollingWindow
```rust
struct RollingWindow {
    buckets: VecDeque<MinuteBucket>,
    window_minutes: usize,
    current_filter: ModelFilter,
}

enum ModelFilter {
    All,
    Specific(String),
}
```