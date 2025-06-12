use crate::models::LogEntry;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use walkdir::WalkDir;

pub struct LogParser {
    claude_dir: String,
    start_date: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>,
}

impl LogParser {
    pub fn new(claude_dir: String) -> Self {
        Self {
            claude_dir,
            start_date: None,
            end_date: None,
        }
    }

    pub fn with_date_range(
        mut self,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    ) -> Self {
        self.start_date = start;
        self.end_date = end;
        self
    }

    pub fn parse_logs(&self) -> Result<Vec<LogEntry>> {
        let expanded_path = shellexpand::tilde(&self.claude_dir).to_string();
        let projects_dir = Path::new(&expanded_path).join("projects");

        if !projects_dir.exists() {
            anyhow::bail!(
                "Claude projects directory not found at: {}",
                projects_dir.display()
            );
        }

        let jsonl_files = self.find_jsonl_files(&projects_dir)?;
        println!("Found {} JSONL files to process", jsonl_files.len());

        let pb = ProgressBar::new(jsonl_files.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} files")
                .unwrap()
                .progress_chars("#>-"),
        );

        let mut all_entries = Vec::new();

        for file_path in jsonl_files {
            pb.inc(1);
            match self.parse_jsonl_file(&file_path) {
                Ok(entries) => all_entries.extend(entries),
                Err(e) => eprintln!("Error parsing {}: {}", file_path.display(), e),
            }
        }

        pb.finish_with_message("Parsing complete");

        // Filter by date range if specified
        let filtered_entries = self.filter_by_date(all_entries);

        // Deduplicate entries
        Ok(self.deduplicate_entries(filtered_entries))
    }

    fn find_jsonl_files(&self, dir: &Path) -> Result<Vec<std::path::PathBuf>> {
        let mut files = Vec::new();

        for entry in WalkDir::new(dir).max_depth(3) {
            let entry = entry?;
            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "jsonl" {
                        files.push(entry.path().to_path_buf());
                    }
                }
            }
        }

        Ok(files)
    }

    fn parse_jsonl_file(&self, path: &Path) -> Result<Vec<LogEntry>> {
        let file = File::open(path).context("Failed to open JSONL file")?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for (line_num, line) in reader.lines().enumerate() {
            let line = line.context("Failed to read line")?;
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<LogEntry>(&line) {
                Ok(entry) => {
                    // Only include assistant messages with usage data
                    if entry.entry_type == "assistant" {
                        if let Some(message) = &entry.message {
                            if message.usage.is_some() {
                                entries.push(entry);
                            }
                        }
                    }
                }
                Err(e) => {
                    // Skip malformed entries silently unless it's a parsing error we care about
                    if line_num < 5 {
                        eprintln!(
                            "Skipping malformed entry in {} line {}: {}",
                            path.display(),
                            line_num + 1,
                            e
                        );
                    }
                }
            }
        }

        Ok(entries)
    }

    fn filter_by_date(&self, entries: Vec<LogEntry>) -> Vec<LogEntry> {
        entries
            .into_iter()
            .filter(|entry| {
                let in_range = match (self.start_date, self.end_date) {
                    (Some(start), Some(end)) => entry.timestamp >= start && entry.timestamp <= end,
                    (Some(start), None) => entry.timestamp >= start,
                    (None, Some(end)) => entry.timestamp <= end,
                    (None, None) => true,
                };

                // Only include entries after June 4, 2024
                let june_4_2024 = DateTime::parse_from_rfc3339("2024-06-04T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc);
                in_range && entry.timestamp > june_4_2024
            })
            .collect()
    }

    fn deduplicate_entries(&self, entries: Vec<LogEntry>) -> Vec<LogEntry> {
        // Group by request_id and keep only the latest entry for each
        let mut request_map: HashMap<String, LogEntry> = HashMap::new();
        let mut no_request_id_entries = Vec::new();

        for entry in entries {
            if let Some(request_id) = &entry.request_id {
                match request_map.get(request_id) {
                    Some(existing) => {
                        if entry.timestamp > existing.timestamp {
                            request_map.insert(request_id.clone(), entry);
                        }
                    }
                    None => {
                        request_map.insert(request_id.clone(), entry);
                    }
                }
            } else {
                // Keep entries without request_id (synthetic messages)
                no_request_id_entries.push(entry);
            }
        }

        // Combine deduplicated entries with no-request-id entries
        let mut result: Vec<LogEntry> = request_map.into_values().collect();
        result.extend(no_request_id_entries);

        // Sort by timestamp
        result.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_filtering() {
        // Add tests here
    }

    #[test]
    fn test_deduplication() {
        // Add tests here
    }
}