use crate::model_name::ModelName;
use crate::parser::LogParser;
use crate::pricing::get_default_pricing;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use std::collections::VecDeque;

use super::data::{RequestInfo, RollingWindow};

#[derive(Debug, Clone, PartialEq)]
pub enum ModelFilter {
    All,
    Specific(ModelName),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TimeRange {
    OneHour,
    TwoHours,
    SixHours,
    TwelveHours,
    TwentyFourHours,
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
    pub claude_dir: String,
    pub model_filter: ModelFilter,
    pub time_range: TimeRange,
    pub rolling_window: RollingWindow,
    pub request_feed: VecDeque<RequestInfo>,
    pub feed_scroll: usize,
    pub feed_paused: bool,
    pub last_update: DateTime<Utc>,
    pub pricing_map: crate::models::PricingMap,
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
            time_range: time_range.clone(),
            rolling_window: RollingWindow::new(time_range.minutes()),
            request_feed: VecDeque::with_capacity(100),
            feed_scroll: 0,
            feed_paused: false,
            last_update: Utc::now(),
            pricing_map: get_default_pricing(),
        }
    }

    pub async fn refresh_data(&mut self) -> Result<()> {
        // Parse logs from the last N hours
        let start_date = Utc::now() - Duration::hours(24); // Always fetch 24h for feed
        let parser = LogParser::new(self.claude_dir.clone())
            .with_date_range(Some(start_date), None);
        
        let entries = parser.parse_logs()?;
        
        // Convert entries to RequestInfo and update rolling window
        self.rolling_window.clear();
        self.request_feed.clear();
        
        for entry in entries {
            if let Some(message) = &entry.message {
                if let Some(usage) = &message.usage {
                    if !message.model.is_synthetic() {
                        let request = RequestInfo {
                            timestamp: entry.timestamp,
                            model: message.model.clone(),
                            input_tokens: usage.input_tokens as u32,
                            output_tokens: usage.output_tokens as u32,
                            cache_tokens: (usage.cache_creation_input_tokens + usage.cache_read_input_tokens) as u32,
                            cost: self.calculate_cost(&message.model, usage),
                        };
                        
                        self.rolling_window.add_request(request.clone());
                        self.request_feed.push_front(request);
                        
                        // Limit feed size
                        if self.request_feed.len() > 100 {
                            self.request_feed.pop_back();
                        }
                    }
                }
            }
        }
        
        self.last_update = Utc::now();
        Ok(())
    }

    fn calculate_cost(&self, model: &ModelName, usage: &crate::models::TokenUsage) -> f64 {
        if let Some(pricing) = crate::pricing::get_model_pricing(&self.pricing_map, model) {
            pricing.calculate_cost(usage)
        } else {
            0.0
        }
    }

    pub async fn on_tick(&mut self) {
        // In a real implementation, we'd check for new JSONL entries
        // For now, just update the timestamp
        self.last_update = Utc::now();
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
}