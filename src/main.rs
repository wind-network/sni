use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::info;

mod config;
mod indexer;
mod network;
mod storage;
mod api;

#[derive(Parser)]
#[command(name = "sni")]
#[command(about = "Solana Network Indexer - Ultra-fast Solana blockchain indexer")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the SNI indexer
    Start {
        /// Configuration file path
        #[arg(short, long, default_value = "sni.toml")]
        config: String,
        /// Enable debug logging
        #[arg(short, long)]
        debug: bool,
    },
    /// Check network health
    Health,
    /// Show version information
    Version,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start { config, debug } => {
            setup_logging(debug)?;
            info!("Starting SNI (Solana Network Indexer)");
            
            let config = config::SniConfig::load(&config)?;
            let mut indexer = indexer::SolanaIndexer::new(config).await?;
            
            indexer.start().await?;
        }
        Commands::Health => {
            println!("Checking Solana network health...");
            network::health_check().await?;
        }
        Commands::Version => {
            println!("SNI v{}", env!("CARGO_PKG_VERSION"));
            println!("Built with Tide engine for ultra-fast Solana indexing");
        }
    }

    Ok(())
}

fn setup_logging(debug: bool) -> Result<()> {
    let level = if debug { "debug" } else { "info" };
    
    tracing_subscriber::fmt()
        .with_env_filter(format!("sni={},tide_core={}", level, level))
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();
        
    Ok(())
}