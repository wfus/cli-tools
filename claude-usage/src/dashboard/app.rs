use crate::model_name::ModelName;
use crate::parser::LogParser;
use crate::pricing::get_default_pricing;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use std::collections::{HashSet, VecDeque};

use super::data::{RequestInfo, RollingWindow};

#[derive(Debug, Clone, PartialEq)]
pub enum ModelFilter {
    All,
    Specific(ModelName),
}

// CLAUDETODO: Add Copy trait to TimeRange since it's a simple enum with no data.
// This would eliminate the need for cloning in App::new()
#[derive(Debug, Clone, PartialEq)]
pub enum TimeRange {
    OneHour,
    TwoHours,
    SixHours,
    TwelveHours,
    TwentyFourHours,
}

// CLAUDETODO: Add Copy trait to ChartType as well since it's a simple enum
#[derive(Debug, Clone, PartialEq)]
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
    // CLAUDETODO: pricing_map is loaded once but never updated. If pricing rarely changes,
    // consider making it a global static or lazy_static to avoid storing in every App instance
    pub pricing_map: crate::models::PricingMap,
    // CLAUDETODO: HashSet<String> for UUIDs is memory-intensive. Consider:
    // 1. Using a bloom filter for probabilistic deduplication
    // 2. Storing only recent UUIDs with a time-based eviction
    // 3. Using u128 or [u8; 16] for UUID storage instead of String
    seen_request_ids: HashSet<String>,
}

impl App {
    pub fn new(claude_dir: String, initial_hours: usize) -> Self {
        let time_range = match initial_hours {
            1 => TimeRange::OneHour,
            2 => TimeRange::TwoHours,
            6 => TimeRange::SixHours,
            12 => TimeRange::TwelveHours,
            24 => TimeRange::TwentyFourHours,
            _ => TimeRange::OneHour,
        };

        Self {
            claude_dir,
            model_filter: ModelFilter::All,
            // CLAUDETODO: Unnecessary clone of time_range enum. Enums implement Copy, so remove .clone()
            time_range: time_range.clone(),
            chart_type: ChartType::Bar,
            rolling_window: RollingWindow::new(time_range.minutes()),
            request_feed: VecDeque::with_capacity(100),
            feed_scroll: 0,
            feed_paused: false,
            last_update: Utc::now(),
            pricing_map: get_default_pricing(),
            // CLAUDETODO: Consider pre-allocating HashSet capacity based on expected request count
            // to reduce rehashing. E.g., HashSet::with_capacity(1000) for typical usage
            seen_request_ids: HashSet::new(),
        }
    }

    pub async fn refresh_data(&mut self) -> Result<()> {
        // CLAUDETODO: This function is marked async but contains no await points. 
        // Either make it sync or add actual async I/O operations (e.g., async file reading)
        
        // Parse logs from the last N hours
        let start_date = Utc::now() - Duration::hours(24); // Always fetch 24h for feed
        // CLAUDETODO: Cloning claude_dir String on every refresh is inefficient. 
        // Consider storing &str in LogParser or using Arc<String> if sharing is needed
        let parser = LogParser::new(self.claude_dir.clone())
            .with_date_range(Some(start_date), None)
            .quiet();
        
        // CLAUDETODO: parse_logs() re-reads and re-parses ALL files every time, even if nothing changed.
        // Consider:
        // 1. Caching file modification times and only parsing changed files
        // 2. Keeping a persistent index of parsed data with timestamps
        // 3. Using inotify/FSEvents to watch for file changes instead of polling
        let entries = parser.parse_logs()?;
        
        // On first load, clear everything and ensure proper sorting
        let is_first_load = self.seen_request_ids.is_empty();
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

    pub async fn on_tick(&mut self) {
        // CLAUDETODO: This async function only calls one other async function. 
        // The async overhead might not be worth it for a simple wrapper.
        // Refresh data from JSONL files
        if let Err(e) = self.refresh_data().await {
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