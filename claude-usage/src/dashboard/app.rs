use crate::file_tracker::FileTracker;
use crate::incremental_parser::IncrementalParsing;
use crate::model_name::ModelName;
use crate::parser::LogParser;
use crate::pricing::get_default_pricing;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;

use super::data::{RequestInfo, RollingWindow};

#[derive(Debug, Clone, PartialEq)]
pub enum ModelFilter {
    All,
    Specific(ModelName),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimeRange {
    OneHour,
    TwoHours,
    SixHours,
    TwelveHours,
    TwentyFourHours,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChartType {
    Bar,
    Line,
}

impl TimeRange {
    pub fn minutes(&self) -> usize {
        match self {
            TimeRange::OneHour => 60,
            TimeRange::TwoHours => 120,
            TimeRange::SixHours => 360,
            TimeRange::TwelveHours => 720,
            TimeRange::TwentyFourHours => 1440,
        }
    }

    pub fn next(&self) -> Self {
        match self {
            TimeRange::OneHour => TimeRange::TwoHours,
            TimeRange::TwoHours => TimeRange::SixHours,
            TimeRange::SixHours => TimeRange::TwelveHours,
            TimeRange::TwelveHours => TimeRange::TwentyFourHours,
            TimeRange::TwentyFourHours => TimeRange::OneHour,
        }
    }
}

pub struct App {
    // CLAUDETODO: Consider using Arc<String> or PathBuf for claude_dir to avoid cloning on every refresh
    pub claude_dir: String,
    pub model_filter: ModelFilter,
    pub time_range: TimeRange,
    pub chart_type: ChartType,
    pub rolling_window: RollingWindow,
    // CLAUDETODO: VecDeque might not be optimal for a feed that's mostly push_front/pop_back.
    // Consider using a ring buffer or a simple Vec with reverse iteration
    pub request_feed: VecDeque<RequestInfo>,
    pub feed_scroll: usize,
    pub feed_paused: bool,
    pub last_update: DateTime<Utc>,
    pub refresh_rate: f64,
    // CLAUDETODO: pricing_map is loaded once but never updated. If pricing rarely changes,
    // consider making it a global static or lazy_static to avoid storing in every App instance
    pub pricing_map: crate::models::PricingMap,
    // CLAUDETODO: HashSet<String> for UUIDs is memory-intensive. Consider:
    // 1. Using a bloom filter for probabilistic deduplication
    // 2. Storing only recent UUIDs with a time-based eviction
    // 3. Using u128 or [u8; 16] for UUID storage instead of String
    seen_request_ids: HashSet<String>,
    _file_tracker: Option<FileTracker>,
    _use_incremental: bool,
}

impl App {
    pub fn new(claude_dir: String, initial_hours: usize, refresh_rate: f64) -> Self {
        let time_range = match initial_hours {
            1 => TimeRange::OneHour,
            2 => TimeRange::TwoHours,
            6 => TimeRange::SixHours,
            12 => TimeRange::TwelveHours,
            24 => TimeRange::TwentyFourHours,
            _ => TimeRange::OneHour,
        };

        // Initialize file tracker for incremental parsing
        let state_dir = PathBuf::from(&claude_dir).join(".claude-usage");
        // Create state directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(&state_dir) {
            eprintln!("Warning: Failed to create state directory: {}", e);
        }
        let state_file = state_dir.join("dashboard-file-tracker.json");
        let file_tracker = FileTracker::with_persistence(state_file);
        
        Self {
            claude_dir,
            model_filter: ModelFilter::All,
            time_range,
            chart_type: ChartType::Bar,
            rolling_window: RollingWindow::new(time_range.minutes()),
            request_feed: VecDeque::with_capacity(100),
            feed_scroll: 0,
            feed_paused: false,
            last_update: Utc::now(),
            refresh_rate,
            pricing_map: get_default_pricing(),
            // CLAUDETODO: Consider pre-allocating HashSet capacity based on expected request count
            // to reduce rehashing. E.g., HashSet::with_capacity(1000) for typical usage
            seen_request_ids: HashSet::new(),
            _file_tracker: Some(file_tracker),
            _use_incremental: true, // Enable by default
        }
    }

    pub fn refresh_data(&mut self) -> Result<()> {
        // Parse logs from the last N hours
        let start_date = Utc::now() - Duration::hours(24); // Always fetch 24h for feed
        let parser = LogParser::new(self.claude_dir.clone())
            .with_date_range(Some(start_date), None)
            .quiet();
        
        // On first load, clear everything and ensure proper sorting
        let is_first_load = self.seen_request_ids.is_empty();
        
        // Use incremental parsing if available, but do full load on first run
        let entries = if let Some(ref mut tracker) = self._file_tracker {
            if self._use_incremental && !is_first_load {
                parser.parse_logs_incremental(tracker)?
            } else {
                // First load or incremental disabled - do full parse
                let entries = parser.parse_logs()?;
                // Update tracker with all files so next refresh is incremental
                if is_first_load && self._use_incremental {
                    // Force tracker to scan all files
                    let _ = parser.parse_logs_incremental(tracker);
                }
                entries
            }
        } else {
            parser.parse_logs()?
        };
        
        if is_first_load {
            self.rolling_window.clear();
            self.request_feed.clear();
        }
        
        // CLAUDETODO: Pre-allocate Vec capacity based on typical new request count
        // to avoid reallocations during push operations
        let mut new_requests = Vec::new();
        
        for entry in entries {
            // Skip if we've already seen this request
            if self.seen_request_ids.contains(&entry.uuid) {
                continue;
            }
            
            if let Some(message) = &entry.message {
                if let Some(usage) = &message.usage {
                    if !message.model.is_synthetic() {
                        let request = RequestInfo {
                            timestamp: entry.timestamp,
                            // CLAUDETODO: Cloning ModelName on every request. Consider using Arc<ModelName>
                            // or storing model as an enum index if the set of models is limited
                            model: message.model.clone(),
                            input_tokens: usage.input_tokens as u32,
                            output_tokens: usage.output_tokens as u32,
                            cache_tokens: (usage.cache_creation_input_tokens + usage.cache_read_input_tokens) as u32,
                            cost: self.calculate_cost(&message.model, usage),
                        };
                        
                        // CLAUDETODO: Cloning RequestInfo here is unnecessary. add_request could take ownership
                        // and new_requests could store references or indices
                        self.rolling_window.add_request(request.clone());
                        new_requests.push(request);
                        // CLAUDETODO: Cloning uuid String for HashSet. Consider using &str with a lifetime
                        // or store hashes of UUIDs instead of full strings
                        self.seen_request_ids.insert(entry.uuid.clone());
                    }
                }
            }
        }
        
        // Sort new requests by timestamp (oldest first)
        new_requests.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        
        // Add new requests to the feed (most recent first)
        if !self.feed_paused {
            // Add in reverse order so newest appears at top
            for request in new_requests.into_iter().rev() {
                self.request_feed.push_front(request);
                
                // Limit feed size
                if self.request_feed.len() > 100 {
                    self.request_feed.pop_back();
                }
            }
        }
        
        // On first load, ensure feed is sorted properly (newest first)
        if is_first_load {
            self.sort_request_feed();
        }
        
        self.last_update = Utc::now();
        Ok(())
    }

    fn calculate_cost(&self, model: &ModelName, usage: &crate::models::TokenUsage) -> f64 {
        // CLAUDETODO: get_model_pricing does HashMap lookups and string comparisons for Unknown models.
        // Consider caching pricing lookups for frequently used models or pre-computing a model->pricing index
        if let Some(pricing) = crate::pricing::get_model_pricing(&self.pricing_map, model) {
            pricing.calculate_cost(usage)
        } else {
            0.0
        }
    }

    pub fn on_tick(&mut self) {
        // Refresh data from JSONL files
        if let Err(e) = self.refresh_data() {
            eprintln!("Error refreshing data: {}", e);
        }
    }

    pub fn cycle_model_filter(&mut self) {
        self.model_filter = match &self.model_filter {
            ModelFilter::All => ModelFilter::Specific(ModelName::Claude4Opus),
            ModelFilter::Specific(ModelName::Claude4Opus) => ModelFilter::Specific(ModelName::Claude4Sonnet),
            ModelFilter::Specific(ModelName::Claude4Sonnet) => ModelFilter::Specific(ModelName::Claude3Haiku),
            ModelFilter::Specific(_) => ModelFilter::All,
        };
    }

    pub fn cycle_time_range(&mut self) {
        self.time_range = self.time_range.next();
        self.rolling_window.set_window_minutes(self.time_range.minutes());
    }

    pub fn toggle_feed_pause(&mut self) {
        self.feed_paused = !self.feed_paused;
        
        // When unpausing, ensure the feed is properly sorted
        if !self.feed_paused {
            self.sort_request_feed();
        }
    }
    
    /// Ensure request feed is sorted with most recent first
    fn sort_request_feed(&mut self) {
        let mut temp: Vec<_> = self.request_feed.drain(..).collect();
        temp.sort_by(|a, b| b.timestamp.cmp(&a.timestamp)); // Reverse order - newest first
        self.request_feed.extend(temp);
    }

    pub fn scroll_feed_up(&mut self) {
        if self.feed_scroll > 0 {
            self.feed_scroll -= 1;
        }
    }

    pub fn scroll_feed_down(&mut self) {
        if self.feed_scroll < self.request_feed.len().saturating_sub(10) {
            self.feed_scroll += 1;
        }
    }

    pub fn toggle_chart_type(&mut self) {
        self.chart_type = match self.chart_type {
            ChartType::Bar => ChartType::Line,
            ChartType::Line => ChartType::Bar,
        };
    }
}