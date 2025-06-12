use anyhow::Result;
use clap::Parser;
use claude_usage::{analyze_usage, cli::Args};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    analyze_usage(args).await?;
    Ok(())
}
