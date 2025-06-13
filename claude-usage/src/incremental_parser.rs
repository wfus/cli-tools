use crate::file_tracker::{FileCheckResult, FileTracker};
use crate::models::LogEntry;
use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;

/// Extension trait for LogParser to add incremental parsing capabilities
pub trait IncrementalParsing {
    fn parse_logs_incremental(&self, tracker: &mut FileTracker) -> Result<Vec<LogEntry>>;
    fn parse_jsonl_file_from_position(
        &self,
        path: &Path,
        start_position: u64,
        start_line: usize,
    ) -> Result<(Vec<LogEntry>, u64, usize)>;
}

impl IncrementalParsing for crate::parser::LogParser {
    fn parse_logs_incremental(&self, tracker: &mut FileTracker) -> Result<Vec<LogEntry>> {
        let expanded_path = shellexpand::tilde(&self.claude_dir).to_string();
        let projects_dir = Path::new(&expanded_path).join("projects");

        if !projects_dir.exists() {
            anyhow::bail!(
                "Claude projects directory not found at: {}",
                projects_dir.display()
            );
        }

        let jsonl_files = self.find_jsonl_files(&projects_dir)?;
        let mut all_entries = Vec::new();
        let mut files_processed = 0;
        let mut bytes_read = 0u64;

        for file_path in jsonl_files {
            match tracker.check_file(&file_path)? {
                FileCheckResult::Unchanged => {
                    // Skip unchanged files
                    continue;
                }
                FileCheckResult::New | FileCheckResult::Rotated => {
                    // Parse entire file for new or rotated files
                    match self.parse_jsonl_file(&file_path) {
                        Ok(entries) => {
                            let file_size = std::fs::metadata(&file_path)?.len();
                            bytes_read += file_size;
                            
                            // Count lines for accurate tracking
                            let line_count = entries.len();
                            tracker.update_state(file_path.clone(), file_size, line_count)?;
                            
                            all_entries.extend(entries);
                            files_processed += 1;
                        }
                        Err(e) => {
                            eprintln!("Error parsing {}: {}", file_path.display(), e);
                            // Remove from tracker if file can't be parsed
                            tracker.remove_file(&file_path);
                        }
                    }
                }
                FileCheckResult::Modified {
                    last_position,
                    last_line,
                } => {
                    // Parse only new content
                    match self.parse_jsonl_file_from_position(&file_path, last_position, last_line)
                    {
                        Ok((entries, new_position, new_line_number)) => {
                            bytes_read += new_position - last_position;
                            tracker.update_state(file_path.clone(), new_position, new_line_number)?;
                            all_entries.extend(entries);
                            files_processed += 1;
                        }
                        Err(e) => {
                            eprintln!(
                                "Error parsing {} from position {}: {}",
                                file_path.display(),
                                last_position,
                                e
                            );
                            // On error, try full reparse next time
                            tracker.remove_file(&file_path);
                        }
                    }
                }
            }
        }

        if !self.quiet && files_processed > 0 {
            println!(
                "Incrementally processed {} files, read {} bytes",
                files_processed,
                format_bytes(bytes_read)
            );
        }

        // Filter by date range if specified
        let filtered_entries = self.filter_by_date(all_entries);

        // Deduplicate entries
        Ok(self.deduplicate_entries(filtered_entries))
    }

    fn parse_jsonl_file_from_position(
        &self,
        path: &Path,
        start_position: u64,
        start_line: usize,
    ) -> Result<(Vec<LogEntry>, u64, usize)> {
        let mut file = File::open(path).context("Failed to open JSONL file")?;
        
        // Seek to the last read position
        file.seek(SeekFrom::Start(start_position))?;
        
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        let mut line_num = start_line;
        let mut current_position = start_position;

        for line_result in reader.lines() {
            let line = line_result.context("Failed to read line")?;
            line_num += 1;
            
            // Update position (approximate - includes line ending)
            current_position += line.len() as u64 + 1; // +1 for newline
            
            if line.trim().is_empty() {
                continue;
            }

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
                    // Log parse errors for debugging
                    if !self.quiet && line_num < start_line + 5 {
                        let error_str = e.to_string();
                        if !error_str.contains("missing field `id`")
                            && !error_str.contains("missing field `uuid`")
                        {
                            eprintln!(
                                "Skipping unexpected entry format in {} line {}: {}",
                                path.display(),
                                line_num,
                                e
                            );
                        }
                    }
                }
            }
        }

        Ok((entries, current_position, line_num))
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_jsonl_file(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
        let file_path = dir.join(name);
        std::fs::write(&file_path, content).unwrap();
        file_path
    }

    #[test]
    fn test_incremental_parsing() {
        let temp_dir = TempDir::new().unwrap();
        let projects_dir = temp_dir.path().join("projects");
        std::fs::create_dir_all(&projects_dir).unwrap();

        // Create initial file
        let initial_content = r#"{"type":"assistant","uuid":"test1","timestamp":"2024-12-01T00:00:00Z","entry_type":"assistant","message":{"model":"claude-4-opus-20250514","usage":{"input_tokens":100,"output_tokens":50}}}
{"type":"assistant","uuid":"test2","timestamp":"2024-12-01T00:01:00Z","entry_type":"assistant","message":{"model":"claude-4-opus-20250514","usage":{"input_tokens":200,"output_tokens":100}}}"#;

        let file_path = create_test_jsonl_file(&projects_dir, "test.jsonl", initial_content);

        let mut tracker = FileTracker::new();
        let parser = crate::parser::LogParser::new(temp_dir.path().to_string_lossy().to_string())
            .quiet();

        // First parse - should read entire file
        let entries1 = parser.parse_logs_incremental(&mut tracker).unwrap();
        assert_eq!(entries1.len(), 2);

        // Parse again without changes - should return empty
        let entries2 = parser.parse_logs_incremental(&mut tracker).unwrap();
        assert_eq!(entries2.len(), 0);

        // Append new content
        let new_content = r#"
{"type":"assistant","uuid":"test3","timestamp":"2024-12-01T00:02:00Z","entry_type":"assistant","message":{"model":"claude-4-opus-20250514","usage":{"input_tokens":150,"output_tokens":75}}}"#;

        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&file_path)
            .unwrap();
        write!(file, "{}", new_content).unwrap();

        // Parse again - should only get new entry
        let entries3 = parser.parse_logs_incremental(&mut tracker).unwrap();
        assert_eq!(entries3.len(), 1);
        assert_eq!(entries3[0].uuid, "test3");
    }

    #[test]
    fn test_file_rotation_handling() {
        let temp_dir = TempDir::new().unwrap();
        let projects_dir = temp_dir.path().join("projects");
        std::fs::create_dir_all(&projects_dir).unwrap();

        let mut tracker = FileTracker::new();
        let parser = crate::parser::LogParser::new(temp_dir.path().to_string_lossy().to_string())
            .quiet();

        // Create and parse initial file
        let initial_content = r#"{"type":"assistant","uuid":"test1","timestamp":"2024-12-01T00:00:00Z","entry_type":"assistant","message":{"model":"claude-4-opus-20250514","usage":{"input_tokens":100,"output_tokens":50}}}"#;
        let file_path = create_test_jsonl_file(&projects_dir, "test.jsonl", initial_content);

        let entries1 = parser.parse_logs_incremental(&mut tracker).unwrap();
        assert_eq!(entries1.len(), 1);

        // Simulate rotation - write shorter content
        let rotated_content = r#"{"type":"assistant","uuid":"test2","timestamp":"2024-12-01T01:00:00Z","entry_type":"assistant","message":{"model":"claude-4-opus-20250514","usage":{"input_tokens":50,"output_tokens":25}}}"#;
        std::fs::write(&file_path, rotated_content).unwrap();

        // Should detect rotation and reparse entire file
        let entries2 = parser.parse_logs_incremental(&mut tracker).unwrap();
        assert_eq!(entries2.len(), 1);
        assert_eq!(entries2[0].uuid, "test2");
    }
}