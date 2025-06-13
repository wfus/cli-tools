use anyhow::Result;
use clap::Parser;
use claude_usage::{analyze_usage, cli::{Args, Cli, Commands}, dashboard};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Some(Commands::Dashboard { refresh, hours, model, claude_dir }) => {
            dashboard::run_dashboard(refresh, hours, model, claude_dir).await?;
        }
        Some(Commands::Show(args)) => {
            analyze_usage(args).await?;
        }
        None => {
            // Default to show command if no subcommand provided
            let args = Args::parse();
            analyze_usage(args).await?;
        }
    }
    
    Ok(())
}
