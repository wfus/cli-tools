use chrono::NaiveDate;
use clap::{Parser, Subcommand, ValueEnum};

fn parse_refresh_rate(s: &str) -> Result<f64, String> {
    s.parse::<f64>()
        .map_err(|_| "Invalid refresh rate".to_string())
        .and_then(|v| {
            if v > 0.0 && v <= 60.0 {
                Ok(v)
            } else {
                Err("Refresh rate must be between 0 and 60 seconds".to_string())
            }
        })
}

#[derive(Parser, Debug)]
#[command(name = "claude-usage")]
#[command(about = "Analyze Claude Code usage and costs from local logs")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Show usage statistics (default)
    #[command(visible_alias = "stats")]
    Show(Args),
    
    /// Launch interactive dashboard
    #[command(visible_aliases = &["dash", "d"])]
    Dashboard {
        /// Refresh interval in seconds (supports decimals, e.g. 0.5)
        #[arg(short, long, default_value = "0.5", value_parser = parse_refresh_rate)]
        refresh: f64,
        
        /// Initial time range in hours
        #[arg(long, default_value = "1")]
        hours: usize,
        
        /// Initial model filter
        #[arg(short, long)]
        model: Option<String>,
        
        /// Path to Claude logs directory
        #[arg(long, default_value = "~/.claude")]
        claude_dir: String,
    },
}

#[derive(Parser, Debug)]
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