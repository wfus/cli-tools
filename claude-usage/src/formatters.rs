use crate::models::{TokenUsage, UsageStats};
use colored::Colorize;
use prettytable::{format, Cell, Row, Table};
use std::collections::HashMap;

pub fn format_table(stats: &[UsageStats], detailed: bool, show_summary: bool) -> String {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);

    // Set headers based on detail level
    if detailed {
        table.set_titles(Row::new(vec![
            Cell::new("Date").style_spec("bFc"),
            Cell::new("Model").style_spec("bFc"),
            Cell::new("Requests").style_spec("bFc"),
            Cell::new("Input").style_spec("bFc"),
            Cell::new("Output").style_spec("bFc"),
            Cell::new("Cache Write").style_spec("bFc"),
            Cell::new("Cache Read").style_spec("bFc"),
            Cell::new("Total Tokens").style_spec("bFc"),
            Cell::new("Cost (USD)").style_spec("bFc"),
        ]));
    } else {
        table.set_titles(Row::new(vec![
            Cell::new("Date").style_spec("bFc"),
            Cell::new("Model").style_spec("bFc"),
            Cell::new("Requests").style_spec("bFc"),
            Cell::new("Total Tokens").style_spec("bFc"),
            Cell::new("Cost (USD)").style_spec("bFc"),
        ]));
    }

    let mut total_cost = 0.0;
    let mut total_requests = 0;
    let mut total_usage = TokenUsage::default();

    for stat in stats {
        total_cost += stat.cost_usd;
        total_requests += stat.request_count;
        total_usage.add(&stat.usage);

        if detailed {
            table.add_row(Row::new(vec![
                Cell::new(&format_date(&stat.date)),
                Cell::new(&stat.model),
                Cell::new(&stat.request_count.to_string()),
                Cell::new(&format_number(stat.usage.input_tokens)),
                Cell::new(&format_number(stat.usage.output_tokens)),
                Cell::new(&format_number(stat.usage.cache_creation_input_tokens)),
                Cell::new(&format_number(stat.usage.cache_read_input_tokens)),
                Cell::new(&format_number(stat.usage.total_tokens())),
                Cell::new(&format!("${:.4}", stat.cost_usd)).style_spec("Fg"),
            ]));
        } else {
            table.add_row(Row::new(vec![
                Cell::new(&format_date(&stat.date)),
                Cell::new(&stat.model),
                Cell::new(&stat.request_count.to_string()),
                Cell::new(&format_number(stat.usage.total_tokens())),
                Cell::new(&format!("${:.4}", stat.cost_usd)).style_spec("Fg"),
            ]));
        }
    }

    // Add summary row if requested
    if show_summary {
        table.add_empty_row();
        if detailed {
            table.add_row(Row::new(vec![
                Cell::new("TOTAL").style_spec("bFy"),
                Cell::new("").style_spec("bFy"),
                Cell::new(&total_requests.to_string()).style_spec("bFy"),
                Cell::new(&format_number(total_usage.input_tokens)).style_spec("bFy"),
                Cell::new(&format_number(total_usage.output_tokens)).style_spec("bFy"),
                Cell::new(&format_number(total_usage.cache_creation_input_tokens))
                    .style_spec("bFy"),
                Cell::new(&format_number(total_usage.cache_read_input_tokens)).style_spec("bFy"),
                Cell::new(&format_number(total_usage.total_tokens())).style_spec("bFy"),
                Cell::new(&format!("${:.4}", total_cost)).style_spec("bFgY"),
            ]));
        } else {
            table.add_row(Row::new(vec![
                Cell::new("TOTAL").style_spec("bFy"),
                Cell::new("").style_spec("bFy"),
                Cell::new(&total_requests.to_string()).style_spec("bFy"),
                Cell::new(&format_number(total_usage.total_tokens())).style_spec("bFy"),
                Cell::new(&format!("${:.4}", total_cost)).style_spec("bFgY"),
            ]));
        }
    }

    table.to_string()
}

pub fn format_csv(stats: &[UsageStats], detailed: bool) -> String {
    let mut csv = String::new();

    // Headers
    if detailed {
        csv.push_str(
            "Date,Model,Requests,Input Tokens,Output Tokens,Cache Write Tokens,Cache Read Tokens,Total Tokens,Cost USD\n"
        );
    } else {
        csv.push_str("Date,Model,Requests,Total Tokens,Cost USD\n");
    }

    // Data rows
    for stat in stats {
        if detailed {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{},{:.4}\n",
                format_date(&stat.date),
                stat.model,
                stat.request_count,
                stat.usage.input_tokens,
                stat.usage.output_tokens,
                stat.usage.cache_creation_input_tokens,
                stat.usage.cache_read_input_tokens,
                stat.usage.total_tokens(),
                stat.cost_usd
            ));
        } else {
            csv.push_str(&format!(
                "{},{},{},{},{:.4}\n",
                format_date(&stat.date),
                stat.model,
                stat.request_count,
                stat.usage.total_tokens(),
                stat.cost_usd
            ));
        }
    }

    csv
}

pub fn format_json(stats: &[UsageStats]) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(stats)
}

pub fn format_markdown(stats: &[UsageStats], detailed: bool, show_summary: bool) -> String {
    let mut md = String::new();

    // Headers
    if detailed {
        md.push_str("| Date | Model | Requests | Input | Output | Cache Write | Cache Read | Total Tokens | Cost (USD) |\n");
        md.push_str("|------|-------|----------|-------|--------|-------------|------------|--------------|------------|\n");
    } else {
        md.push_str("| Date | Model | Requests | Total Tokens | Cost (USD) |\n");
        md.push_str("|------|-------|----------|--------------|------------|\n");
    }

    let mut total_cost = 0.0;
    let mut total_requests = 0;
    let mut total_usage = TokenUsage::default();

    // Data rows
    for stat in stats {
        total_cost += stat.cost_usd;
        total_requests += stat.request_count;
        total_usage.add(&stat.usage);

        if detailed {
            md.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | ${:.4} |\n",
                format_date(&stat.date),
                stat.model,
                stat.request_count,
                format_number(stat.usage.input_tokens),
                format_number(stat.usage.output_tokens),
                format_number(stat.usage.cache_creation_input_tokens),
                format_number(stat.usage.cache_read_input_tokens),
                format_number(stat.usage.total_tokens()),
                stat.cost_usd
            ));
        } else {
            md.push_str(&format!(
                "| {} | {} | {} | {} | ${:.4} |\n",
                format_date(&stat.date),
                stat.model,
                stat.request_count,
                format_number(stat.usage.total_tokens()),
                stat.cost_usd
            ));
        }
    }

    // Summary row
    if show_summary {
        if detailed {
            md.push_str(&format!(
                "| **TOTAL** | | **{}** | **{}** | **{}** | **{}** | **{}** | **{}** | **${:.4}** |\n",
                total_requests,
                format_number(total_usage.input_tokens),
                format_number(total_usage.output_tokens),
                format_number(total_usage.cache_creation_input_tokens),
                format_number(total_usage.cache_read_input_tokens),
                format_number(total_usage.total_tokens()),
                total_cost
            ));
        } else {
            md.push_str(&format!(
                "| **TOTAL** | | **{}** | **{}** | **${:.4}** |\n",
                total_requests,
                format_number(total_usage.total_tokens()),
                total_cost
            ));
        }
    }

    md
}

fn format_date(date: &chrono::DateTime<chrono::Utc>) -> String {
    date.format("%Y-%m-%d").to_string()
}

fn format_number(num: u64) -> String {
    let num_str = num.to_string();
    let mut result = String::new();
    let mut count = 0;

    for ch in num_str.chars().rev() {
        if count == 3 {
            result.push(',');
            count = 0;
        }
        result.push(ch);
        count += 1;
    }

    result.chars().rev().collect()
}

pub fn print_summary(stats: &[UsageStats]) {
    println!("\n{}", "=== Usage Summary ===".bright_cyan().bold());

    let total_cost: f64 = stats.iter().map(|s| s.cost_usd).sum();
    let total_requests: u64 = stats.iter().map(|s| s.request_count).sum();
    let mut total_usage = TokenUsage::default();
    for stat in stats {
        total_usage.add(&stat.usage);
    }

    // Group by model
    let mut model_stats: HashMap<String, (u64, TokenUsage, f64)> = HashMap::new();
    for stat in stats {
        let entry = model_stats
            .entry(stat.model.clone())
            .or_insert((0, TokenUsage::default(), 0.0));
        entry.0 += stat.request_count;
        entry.1.add(&stat.usage);
        entry.2 += stat.cost_usd;
    }

    println!("\n{}", "Overall Statistics:".yellow());
    println!("  Total Requests: {}", format_number(total_requests).green());
    println!(
        "  Total Tokens: {}",
        format_number(total_usage.total_tokens()).green()
    );
    println!("  Total Cost: {}", format!("${:.4}", total_cost).green().bold());

    println!("\n{}", "Token Breakdown:".yellow());
    println!(
        "  Input Tokens: {}",
        format_number(total_usage.input_tokens).cyan()
    );
    println!(
        "  Output Tokens: {}",
        format_number(total_usage.output_tokens).cyan()
    );
    println!(
        "  Cache Write Tokens: {}",
        format_number(total_usage.cache_creation_input_tokens).cyan()
    );
    println!(
        "  Cache Read Tokens: {}",
        format_number(total_usage.cache_read_input_tokens).cyan()
    );

    println!("\n{}", "By Model:".yellow());
    let mut model_vec: Vec<_> = model_stats.into_iter().collect();
    model_vec.sort_by(|a, b| b.1 .2.partial_cmp(&a.1 .2).unwrap());

    for (model, (requests, usage, cost)) in model_vec {
        println!("\n  {}:", model.bright_blue());
        println!("    Requests: {}", format_number(requests));
        println!("    Tokens: {}", format_number(usage.total_tokens()));
        println!("    Cost: {}", format!("${:.4}", cost).green());
    }
}