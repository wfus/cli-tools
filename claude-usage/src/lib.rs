pub mod cli;
pub mod formatters;
pub mod models;
pub mod parser;
pub mod pricing;

use anyhow::Result;
use chrono::{Datelike, TimeZone, Utc};
use cli::{GroupBy, OutputFormat};
use models::{LogEntry, TokenUsage, UsageStats};
use parser::LogParser;
use pricing::{get_default_pricing, get_model_pricing};
use std::collections::HashMap;

pub async fn analyze_usage(args: cli::Args) -> Result<()> {
    // Get pricing information
    let pricing_map = if args.refresh_pricing {
        pricing::fetch_latest_pricing().await?
    } else {
        get_default_pricing()
    };

    // Parse date range
    let start_date = args
        .start_date
        .map(|d| Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0).unwrap()));
    let end_date = args
        .end_date
        .map(|d| Utc.from_utc_datetime(&d.and_hms_opt(23, 59, 59).unwrap()));

    // Parse logs
    let parser = LogParser::new(args.claude_dir.clone()).with_date_range(start_date, end_date);
    let entries = parser.parse_logs()?;

    if entries.is_empty() {
        println!("No usage data found for the specified date range.");
        return Ok(());
    }

    println!("Processed {} unique requests", entries.len());

    // Group and calculate stats
    let stats = calculate_stats(entries, &args.group_by, args.model, &pricing_map)?;

    if stats.is_empty() {
        println!("No usage data matches the specified filters.");
        return Ok(());
    }

    // Format and display output
    match args.format {
        OutputFormat::Table => {
            println!("{}", formatters::format_table(&stats, args.detailed, args.summary));
        }
        OutputFormat::Csv => {
            println!("{}", formatters::format_csv(&stats, args.detailed));
        }
        OutputFormat::Json => {
            println!("{}", formatters::format_json(&stats)?);
        }
        OutputFormat::Markdown => {
            println!("{}", formatters::format_markdown(&stats, args.detailed, args.summary));
        }
    }

    // Print summary if requested
    if args.summary && args.format != OutputFormat::Table {
        formatters::print_summary(&stats);
    }

    Ok(())
}

fn calculate_stats(
    entries: Vec<LogEntry>,
    group_by: &GroupBy,
    model_filter: Option<String>,
    pricing_map: &HashMap<String, models::ModelPricing>,
) -> Result<Vec<UsageStats>> {
    let mut grouped_data: HashMap<String, (String, Vec<LogEntry>)> = HashMap::new();

    for entry in entries {
        // Skip if no message or usage data
        let message = match &entry.message {
            Some(m) => m,
            None => continue,
        };

        let _usage = match &message.usage {
            Some(u) => u,
            None => continue,
        };

        // Apply model filter if specified
        if let Some(filter) = &model_filter {
            if !message.model.contains(filter) {
                continue;
            }
        }

        // Skip synthetic models
        if message.model == "<synthetic>" {
            continue;
        }

        // Generate grouping key
        let (key, model) = match group_by {
            GroupBy::Day => (
                entry.timestamp.date_naive().to_string(),
                "all".to_string(),
            ),
            GroupBy::Week => {
                let week = entry.timestamp.iso_week();
                (
                    format!("{}-W{:02}", week.year(), week.week()),
                    "all".to_string(),
                )
            }
            GroupBy::Month => (
                format!("{}-{:02}", entry.timestamp.year(), entry.timestamp.month()),
                "all".to_string(),
            ),
            GroupBy::Model => ("all-time".to_string(), message.model.clone()),
            GroupBy::ModelDay => (
                format!("{}-{}", entry.timestamp.date_naive(), message.model),
                message.model.clone(),
            ),
            GroupBy::None => ("all-time".to_string(), "all".to_string()),
        };

        grouped_data
            .entry(key.clone())
            .or_insert((model, Vec::new()))
            .1
            .push(entry);
    }

    // Calculate stats for each group
    let mut stats = Vec::new();

    for (_key, (model, entries)) in grouped_data {
        let mut total_usage = TokenUsage::default();
        let mut request_count = 0;
        let date = entries[0].timestamp;

        for entry in &entries {
            if let Some(message) = &entry.message {
                if let Some(usage) = &message.usage {
                    total_usage.add(usage);
                    request_count += 1;
                }
            }
        }

        // Calculate cost
        let actual_model = if model == "all" {
            // Find the most common model in this group
            let mut model_counts: HashMap<String, u32> = HashMap::new();
            for entry in &entries {
                if let Some(message) = &entry.message {
                    *model_counts.entry(message.model.clone()).or_insert(0) += 1;
                }
            }
            model_counts
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .map(|(model, _)| model)
                .unwrap_or_else(|| "unknown".to_string())
        } else {
            model.clone()
        };

        let cost = if let Some(pricing) = get_model_pricing(pricing_map, &actual_model) {
            pricing.calculate_cost(&total_usage)
        } else {
            eprintln!("Warning: No pricing found for model: {}", actual_model);
            0.0
        };

        stats.push(UsageStats {
            model: if model == "all" { actual_model } else { model },
            date,
            usage: total_usage,
            request_count,
            cost_usd: cost,
        });
    }

    // Sort by date
    stats.sort_by(|a, b| a.date.cmp(&b.date));

    Ok(stats)
}