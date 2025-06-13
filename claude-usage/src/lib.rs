pub mod cli;
pub mod formatters;
pub mod model_name;
pub mod models;
pub mod parser;
pub mod pricing;

use anyhow::Result;
use chrono::{Datelike, TimeZone, Utc};
use cli::{GroupBy, OutputFormat};
use model_name::ModelName;
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
    pricing_map: &HashMap<ModelName, models::ModelPricing>,
) -> Result<Vec<UsageStats>> {
    let mut grouped_data: HashMap<String, (ModelName, Vec<LogEntry>)> = HashMap::new();

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
            let model_str = message.model.canonical_string();
            if !model_str.contains(filter) && message.model.to_string() != *filter {
                continue;
            }
        }

        // Skip synthetic models
        if message.model.is_synthetic() {
            continue;
        }

        // Generate grouping key
        let (key, model) = match group_by {
            GroupBy::Day => (
                entry.timestamp.date_naive().to_string(),
                ModelName::Unknown("all".to_string()),
            ),
            GroupBy::Week => {
                let week = entry.timestamp.iso_week();
                (
                    format!("{}-W{:02}", week.year(), week.week()),
                    ModelName::Unknown("all".to_string()),
                )
            }
            GroupBy::Month => (
                format!("{}-{:02}", entry.timestamp.year(), entry.timestamp.month()),
                ModelName::Unknown("all".to_string()),
            ),
            GroupBy::Model => ("all-time".to_string(), message.model.clone()),
            GroupBy::ModelDay => (
                format!("{}-{}", entry.timestamp.date_naive(), message.model),
                message.model.clone(),
            ),
            GroupBy::None => ("all-time".to_string(), ModelName::Unknown("all".to_string())),
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
        let mut total_cost = 0.0;
        let date = entries[0].timestamp;

        // When aggregating across all models, calculate cost per entry
        if matches!(&model, ModelName::Unknown(s) if s == "all") {
            for entry in &entries {
                if let Some(message) = &entry.message {
                    if let Some(usage) = &message.usage {
                        total_usage.add(usage);
                        request_count += 1;
                        
                        // Calculate cost for this specific model
                        if let Some(pricing) = get_model_pricing(pricing_map, &message.model) {
                            total_cost += pricing.calculate_cost(usage);
                        } else {
                            eprintln!("Warning: No pricing found for model: {}", message.model);
                        }
                    }
                }
            }
        } else {
            // For model-specific grouping, use the single model pricing
            for entry in &entries {
                if let Some(message) = &entry.message {
                    if let Some(usage) = &message.usage {
                        total_usage.add(usage);
                        request_count += 1;
                    }
                }
            }
            
            if let Some(pricing) = get_model_pricing(pricing_map, &model) {
                total_cost = pricing.calculate_cost(&total_usage);
            } else {
                eprintln!("Warning: No pricing found for model: {}", model);
            }
        }

        stats.push(UsageStats {
            model: model.clone(),
            date,
            usage: total_usage,
            request_count,
            cost_usd: total_cost,
        });
    }

    // Sort by date
    stats.sort_by(|a, b| a.date.cmp(&b.date));

    Ok(stats)
}