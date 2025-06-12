use chrono::NaiveDate;
use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "claude-usage")]
#[command(about = "Analyze Claude Code usage and costs from local logs")]
#[command(version)]
pub struct Args {
    /// Start date for analysis (YYYY-MM-DD)
    #[arg(short, long)]
    pub start_date: Option<NaiveDate>,

    /// End date for analysis (YYYY-MM-DD)
    #[arg(short, long)]
    pub end_date: Option<NaiveDate>,

    /// Group results by
    #[arg(short, long, value_enum, default_value = "day")]
    pub group_by: GroupBy,

    /// Filter by model name
    #[arg(short, long)]
    pub model: Option<String>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "table")]
    pub format: OutputFormat,

    /// Show detailed token breakdown
    #[arg(short, long)]
    pub detailed: bool,

    /// Path to Claude logs directory
    #[arg(long, default_value = "~/.claude")]
    pub claude_dir: String,

    /// Refresh pricing information from Anthropic API
    #[arg(long)]
    pub refresh_pricing: bool,

    /// Show summary statistics
    #[arg(long)]
    pub summary: bool,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum GroupBy {
    Day,
    Week,
    Month,
    Model,
    ModelDay,
    None,
}

#[derive(Debug, Clone, ValueEnum, PartialEq)]
pub enum OutputFormat {
    Table,
    Json,
    Csv,
    Markdown,
}