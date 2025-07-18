//! Data structures and rolling window implementation for the dashboard.
//!
//! This module provides the core data structures for tracking Claude usage statistics
//! in time-bucketed windows. The `RollingWindow` maintains minute-by-minute data and
//! provides aggregated stats for different time ranges (1h, 5h, 24h, 2d, 7d).

use crate::model_name::ModelName;
use chrono::{DateTime, Duration, Timelike, Utc};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone)]
pub struct TimeRangeStats {
    pub requests: u32,
    pub tokens: u64,
    pub cost: f64,
    pub model_costs: HashMap<String, f64>,
}

#[derive(Debug, Clone)]
pub struct RequestInfo {
    pub timestamp: DateTime<Utc>,
    pub model: ModelName,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_tokens: u32,
    pub cost: f64,
}

#[derive(Debug, Clone)]
pub struct MinuteBucket {
    pub timestamp: DateTime<Utc>,
    pub requests: Vec<RequestInfo>,
    pub total_cost: f64,
    pub model_costs: HashMap<String, f64>,
}

impl MinuteBucket {
    pub fn new(timestamp: DateTime<Utc>) -> Self {
        Self {
            timestamp,
            requests: Vec::new(),
            total_cost: 0.0,
            model_costs: HashMap::new(),
        }
    }

    pub fn add_request(&mut self, request: RequestInfo) {
        let model_key = request.model.family().to_string();
        *self.model_costs.entry(model_key).or_insert(0.0) += request.cost;
        self.total_cost += request.cost;
        self.requests.push(request);
    }
}

pub struct RollingWindow {
    pub buckets: VecDeque<MinuteBucket>,
    pub window_minutes: usize,
}

impl RollingWindow {
    pub fn new(window_minutes: usize) -> Self {
        // Always allocate capacity for at least 7 days of data
        let min_capacity = 168 * 60; // 7 days in minutes
        let capacity = window_minutes.max(min_capacity);
        
        Self {
            buckets: VecDeque::with_capacity(capacity),
            window_minutes,
        }
    }

    pub fn clear(&mut self) {
        self.buckets.clear();
    }

    pub fn set_window_minutes(&mut self, minutes: usize) {
        self.window_minutes = minutes;
        // Trim buckets if needed
        while self.buckets.len() > minutes {
            self.buckets.pop_front();
        }
    }

    pub fn add_request(&mut self, request: RequestInfo) {
        // Round timestamp to minute
        let minute = request.timestamp
            .with_second(0).unwrap()
            .with_nanosecond(0).unwrap();

        // Find or create bucket for this minute
        let bucket_pos = self.buckets.iter().position(|b| b.timestamp == minute);
        
        match bucket_pos {
            Some(pos) => {
                self.buckets[pos].add_request(request);
            }
            None => {
                // Create new bucket
                let mut bucket = MinuteBucket::new(minute);
                bucket.add_request(request);
                
                // Insert in correct position to maintain order
                let insert_pos = self.buckets.iter().position(|b| b.timestamp > minute)
                    .unwrap_or(self.buckets.len());
                self.buckets.insert(insert_pos, bucket);
                
                // Trim old buckets
                self.trim_old_buckets();
            }
        }
    }

    fn trim_old_buckets(&mut self) {
        // Always keep at least 7 days of data for the stats panels
        // This ensures all time ranges (1h, 5h, 24h, 2d, 7d) work correctly regardless of chart view
        let min_retention_hours = 168; // 7 days
        let min_retention_minutes = min_retention_hours * 60;
        
        // Use the larger of the window size or minimum retention
        let retention_minutes = self.window_minutes.max(min_retention_minutes);
        
        // Add a small buffer to ensure stats calculations at boundaries don't miss data
        let buffer_minutes = 5;
        let cutoff = Utc::now() - Duration::minutes((retention_minutes + buffer_minutes) as i64);
        
        while let Some(bucket) = self.buckets.front() {
            if bucket.timestamp < cutoff {
                self.buckets.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn get_minute_costs(&self, model_filter: Option<&ModelName>) -> Vec<(DateTime<Utc>, f64)> {
        self.buckets.iter().map(|bucket| {
            let cost = match model_filter {
                Some(model) => bucket.model_costs.get(model.family()).copied().unwrap_or(0.0),
                None => bucket.total_cost,
            };
            (bucket.timestamp, cost)
        }).collect()
    }

    /// Get stats for a specific time range
    fn get_time_range_stats(&self, hours: i64, model_filter: Option<&ModelName>) -> TimeRangeStats {
        let cutoff = Utc::now() - Duration::hours(hours);
        let mut total_requests = 0u32;
        let mut total_tokens = 0u64;
        let mut total_cost = 0.0;
        let mut model_costs = HashMap::new();

        for bucket in &self.buckets {
            if bucket.timestamp >= cutoff {
                for request in &bucket.requests {
                    if model_filter.is_none() || request.model.family() == model_filter.unwrap().family() {
                        total_requests += 1;
                        total_tokens += (request.input_tokens + request.output_tokens + request.cache_tokens) as u64;
                        total_cost += request.cost;
                        
                        // Also add to model breakdown (respecting filter)
                        let model_key = request.model.family().to_string();
                        *model_costs.entry(model_key).or_insert(0.0) += request.cost;
                    }
                }
            }
        }

        TimeRangeStats {
            requests: total_requests,
            tokens: total_tokens,
            cost: total_cost,
            model_costs,
        }
    }
    
    pub fn get_current_hour_stats(&self, model_filter: Option<&ModelName>) -> TimeRangeStats {
        self.get_time_range_stats(1, model_filter)
    }

    pub fn get_5h_stats(&self, model_filter: Option<&ModelName>) -> TimeRangeStats {
        self.get_time_range_stats(5, model_filter)
    }

    pub fn get_24h_stats(&self, model_filter: Option<&ModelName>) -> TimeRangeStats {
        self.get_time_range_stats(24, model_filter)
    }

    pub fn get_2d_stats(&self, model_filter: Option<&ModelName>) -> TimeRangeStats {
        self.get_time_range_stats(48, model_filter)
    }

    pub fn get_7d_stats(&self, model_filter: Option<&ModelName>) -> TimeRangeStats {
        self.get_time_range_stats(168, model_filter)
    }
}