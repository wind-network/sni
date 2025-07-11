use std::{sync::Arc, time::{Duration, Instant}};
use tokio::time::sleep;
use tide_core::TideEngine;
use anyhow::Result;
use tracing::{info, error, debug};

// Local data structures since tide-common isn't available
#[derive(Debug, Clone)]
pub struct TideData {
    pub slot: u64,
    pub block_hash: String,
    pub timestamp: i64,
}

use crate::config::SniConfig;
use crate::network::{NetworkMonitor, ValidatorTracker};
use crate::storage::{StorageManager, IndexedData};

pub struct SolanaIndexer {
    config: SniConfig,
    tide_engine: TideEngine,
    network_monitor: NetworkMonitor,
    validator_tracker: ValidatorTracker,
    storage: StorageManager,
    stats: Arc<IndexerStats>,
    running: Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Debug, Default)]
pub struct IndexerStats {
    pub blocks_processed: std::sync::atomic::AtomicU64,
    pub transactions_processed: std::sync::atomic::AtomicU64,
    pub accounts_updated: std::sync::atomic::AtomicU64,
    pub processing_latency_ms: std::sync::atomic::AtomicU64,
    pub started_at: std::sync::OnceLock<Instant>,
}

impl SolanaIndexer {
    pub async fn new(config: SniConfig) -> Result<Self> {
        info!("Initializing SNI with config: {:?}", config);
        
        // Create a default TideConfig since we don't have tide_config available
        let default_config = Default::default();
        let tide_engine = TideEngine::new(default_config).await
            .map_err(|e| anyhow::anyhow!("Failed to create TideEngine: {}", e))?;
        
        let storage = StorageManager::new(&config.storage).await?;
        let network_monitor = NetworkMonitor::new(&config.network).await?;
        let validator_tracker = ValidatorTracker::new().await?;
        let stats = Arc::new(IndexerStats::default());
        
        Ok(Self {
            config,
            tide_engine,
            network_monitor,
            validator_tracker,
            storage,
            stats,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting SNI indexer");
        self.stats.started_at.set(Instant::now()).map_err(|_| anyhow::anyhow!("Already started"))?;
        self.running.store(true, std::sync::atomic::Ordering::SeqCst);

        tokio::try_join!(
            self.run_tide_engine(),
            self.run_network_monitor(),
            self.run_stats_reporter(),
        )?;

        Ok(())
    }

    async fn run_tide_engine(&self) -> Result<()> {
        info!("Starting Tide engine");
        self.tide_engine.start().await
            .map_err(|e| anyhow::anyhow!("Failed to start TideEngine: {}", e))
    }

    // TODO: Implement data processing when TideEngine provides a data channel
    // async fn run_data_processor(&self, receiver: Receiver<TideData>) -> Result<()> {
    //     info!("Starting data processor");
    //     
    //     while self.running.load(std::sync::atomic::Ordering::SeqCst) {
    //         match receiver.try_recv() {
    //             Ok(data) => {
    //                 let start = Instant::now();
    //                 self.process_tide_data(data).await?;
    //                 
    //                 let latency = start.elapsed().as_millis() as u64;
    //                 self.stats.processing_latency_ms.store(latency, std::sync::atomic::Ordering::Relaxed);
    //             }
    //             Err(crossbeam_channel::TryRecvError::Empty) => {
    //                 sleep(Duration::from_millis(1)).await;
    //             }
    //             Err(crossbeam_channel::TryRecvError::Disconnected) => {
    //                 warn!("Tide data channel disconnected");
    //                 break;
    //             }
    //         }
    //     }
    //     
    //     Ok(())
    // }

    async fn process_tide_data(&self, data: TideData) -> Result<()> {
        let TideData { slot, block_hash, timestamp } = data;
        debug!("Processing data for slot {}", slot);
        
        let indexed_data = IndexedData::Block {
            slot: slot,
            parent_slot: 0, // Placeholder, not available in TideData
            height: 0, // Placeholder, not available in TideData
            timestamp: timestamp,
            blockhash: block_hash,
            transactions_count: 0, // Placeholder, not available in TideData
        };
        
        self.storage.store(indexed_data).await?;
        self.stats.blocks_processed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        Ok(())
    }

    async fn run_network_monitor(&self) -> Result<()> {
        info!("Starting network monitor");
        
        while self.running.load(std::sync::atomic::Ordering::SeqCst) {
            if let Err(e) = self.network_monitor.check_health().await {
                error!("Network health check failed: {}", e);
            }
            
            if let Err(e) = self.validator_tracker.update_validator_info().await {
                error!("Validator tracking update failed: {}", e);
            }
            
            sleep(Duration::from_secs(30)).await;
        }
        
        Ok(())
    }

    async fn run_stats_reporter(&self) -> Result<()> {
        info!("Starting stats reporter");
        
        while self.running.load(std::sync::atomic::Ordering::SeqCst) {
            let blocks = self.stats.blocks_processed.load(std::sync::atomic::Ordering::Relaxed);
            let txs = self.stats.transactions_processed.load(std::sync::atomic::Ordering::Relaxed);
            let accounts = self.stats.accounts_updated.load(std::sync::atomic::Ordering::Relaxed);
            let latency = self.stats.processing_latency_ms.load(std::sync::atomic::Ordering::Relaxed);
            
            let uptime = self.stats.started_at.get()
                .map(|start| start.elapsed().as_secs())
                .unwrap_or(0);
            
            info!(
                "SNI Stats - Uptime: {}s | Blocks: {} | Transactions: {} | Accounts: {} | Latency: {}ms",
                uptime, blocks, txs, accounts, latency
            );
            
            sleep(Duration::from_secs(60)).await;
        }
        
        Ok(())
    }

    pub fn stop(&self) {
        info!("Stopping SNI indexer");
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
        self.tide_engine.stop();
    }
}