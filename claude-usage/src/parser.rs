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
    // CLAUDETODO: Consider using &str or Path instead of String to avoid unnecessary allocations
    // when the claude_dir is only read and not modified. This would require lifetime parameters.
    claude_dir: String,
    start_date: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>,
    quiet: bool,
}

impl LogParser {
    pub fn new(claude_dir: String) -> Self {
        Self {
            claude_dir,
            start_date: None,
            end_date: None,
            quiet: false,
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

    pub fn quiet(mut self) -> Self {
        self.quiet = true;
        self
    }

    pub fn parse_logs(&self) -> Result<Vec<LogEntry>> {
        // CLAUDETODO: shellexpand::tilde returns a Cow<str>, but we're immediately calling to_string()
        // which causes an unnecessary allocation. Consider using the Cow directly or into_owned()
        // only when necessary.
        let expanded_path = shellexpand::tilde(&self.claude_dir).to_string();
        let projects_dir = Path::new(&expanded_path).join("projects");

        if !projects_dir.exists() {
            anyhow::bail!(
                "Claude projects directory not found at: {}",
                projects_dir.display()
            );
        }

        let jsonl_files = self.find_jsonl_files(&projects_dir)?;
        if !self.quiet {
            println!("Found {} JSONL files to process", jsonl_files.len());
        }

        let pb = if self.quiet {
            ProgressBar::hidden()
        } else {
            ProgressBar::new(jsonl_files.len() as u64)
        };
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} files")
                .unwrap()
                .progress_chars("#>-"),
        );

        // CLAUDETODO: Consider pre-allocating Vec capacity based on estimated entries per file
        // to reduce reallocations during extend operations. Could sample first few files to estimate.
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
                        // CLAUDETODO: entry.path() returns a &Path, but to_path_buf() clones it.
                        // Since we're collecting paths anyway, this is necessary, but consider
                        // using entry.into_path() to avoid the clone if WalkDir allows it.
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

            // CLAUDETODO: This parses the JSON twice - once as serde_json::Value and once as LogEntry.
            // This is inefficient. Consider either:
            // 1. Parse once as Value and check fields before deserializing to LogEntry
            // 2. Add a #[serde(tag = "type")] enum to handle different entry types
            // 3. Use serde's untagged enum feature to try different formats
            // First check if this is a known alternative format
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&line) {
                // Skip summary entries - they don't contain usage data
                if json_value.get("type").and_then(|t| t.as_str()) == Some("summary") {
                    continue;
                }
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
                    // Only warn about parsing errors for non-summary entries
                    // and only for the first few lines to avoid spam
                    if !self.quiet && line_num < 5 {
                        // Check if it's a known issue (missing fields in older formats)
                        // CLAUDETODO: Calling to_string() on error is expensive. Consider using
                        // pattern matching on the error type or checking error kind directly.
                        let error_str = e.to_string();
                        if !error_str.contains("missing field `id`") && !error_str.contains("missing field `uuid`") {
                            eprintln!(
                                "Skipping unexpected entry format in {} line {}: {}",
                                path.display(),
                                line_num + 1,
                                e
                            );
                        }
                    }
                }
            }
        }

        Ok(entries)
    }

    fn filter_by_date(&self, entries: Vec<LogEntry>) -> Vec<LogEntry> {
        // CLAUDETODO: This function takes ownership of entries Vec unnecessarily.
        // Consider taking &[LogEntry] and returning Vec<LogEntry> to avoid moving data.
        // Also, the june_4_2024 date is computed for every entry - should be a const or lazy_static.
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
                // CLAUDETODO: This DateTime is parsed on every iteration! Move outside the loop.
                let june_4_2024 = DateTime::parse_from_rfc3339("2024-06-04T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc);
                in_range && entry.timestamp > june_4_2024
            })
            .collect()
    }

    fn deduplicate_entries(&self, entries: Vec<LogEntry>) -> Vec<LogEntry> {
        // CLAUDETODO: This function also takes ownership unnecessarily. Consider using &[LogEntry].
        // Group by request_id and keep only the latest entry for each
        // CLAUDETODO: Consider pre-allocating HashMap capacity based on entries.len() to reduce rehashing.
        let mut request_map: HashMap<String, LogEntry> = HashMap::new();
        let mut no_request_id_entries = Vec::new();

        for entry in entries {
            if let Some(request_id) = &entry.request_id {
                // CLAUDETODO: Cloning request_id on every insert is inefficient. Consider using
                // entry API: request_map.entry(request_id.clone()).and_modify(|e| {...}).or_insert(entry)
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
        // CLAUDETODO: Pre-allocate capacity for result Vec to avoid reallocations during extend
        let mut result: Vec<LogEntry> = request_map.into_values().collect();
        result.extend(no_request_id_entries);

        // Sort by timestamp
        // CLAUDETODO: Consider using sort_unstable_by for better performance if stable sort isn't needed
        result.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        result
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_date_filtering() {
        // Add tests here
    }

    #[test]
    fn test_deduplication() {
        // Add tests here
    }
}