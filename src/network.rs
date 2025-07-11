use anyhow::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, debug};

use crate::config::NetworkConfig;

#[derive(Clone)]  // Remove Debug since RpcClient doesn't implement it
pub struct NetworkMonitor {
    rpc_client: Arc<RpcClient>,
    config: NetworkConfig,
    last_health_check: Arc<std::sync::RwLock<Option<Instant>>>,
    network_stats: Arc<NetworkStats>,
}

#[derive(Debug, Default)]
pub struct NetworkStats {
    pub slot_height: std::sync::atomic::AtomicU64,
    pub epoch: std::sync::atomic::AtomicU64,
    pub transaction_count: std::sync::atomic::AtomicU64,
    pub average_slot_time: std::sync::atomic::AtomicU64,
    pub active_validators: std::sync::atomic::AtomicU64,
}

#[derive(Debug, Clone)]
pub struct ValidatorTracker {
    validators: Arc<DashMap<Pubkey, ValidatorInfo>>,
    last_update: Arc<std::sync::RwLock<Option<Instant>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorInfo {
    pub vote_account: Pubkey,
    pub identity: Pubkey,
    pub commission: u8,
    pub last_vote: u64,
    pub activated_stake: u64,
    pub delinquent: bool,
}

impl NetworkMonitor {
    pub async fn new(config: &NetworkConfig) -> Result<Self> {
        let rpc_client = Arc::new(RpcClient::new(config.rpc_url.clone()));
        
        Ok(Self {
            rpc_client,
            config: config.clone(),
            last_health_check: Arc::new(std::sync::RwLock::new(None)),
            network_stats: Arc::new(NetworkStats::default()),
        })
    }

    pub async fn check_health(&self) -> Result<()> {
        let start = Instant::now();
        
        let slot = self.rpc_client.get_slot()?;
        self.network_stats.slot_height.store(slot, std::sync::atomic::Ordering::Relaxed);
        
        let epoch_info = self.rpc_client.get_epoch_info()?;
        self.network_stats.epoch.store(epoch_info.epoch, std::sync::atomic::Ordering::Relaxed);
        
        let transaction_count = self.rpc_client.get_transaction_count()?;
        self.network_stats.transaction_count.store(transaction_count, std::sync::atomic::Ordering::Relaxed);
        
        let health_check_time = start.elapsed().as_millis();
        debug!("Network health check completed in {}ms", health_check_time);
        
        *self.last_health_check.write().unwrap() = Some(Instant::now());
        
        Ok(())
    }

    pub fn get_stats(&self) -> NetworkStats {
        NetworkStats {
            slot_height: std::sync::atomic::AtomicU64::new(
                self.network_stats.slot_height.load(std::sync::atomic::Ordering::Relaxed)
            ),
            epoch: std::sync::atomic::AtomicU64::new(
                self.network_stats.epoch.load(std::sync::atomic::Ordering::Relaxed)
            ),
            transaction_count: std::sync::atomic::AtomicU64::new(
                self.network_stats.transaction_count.load(std::sync::atomic::Ordering::Relaxed)
            ),
            average_slot_time: std::sync::atomic::AtomicU64::new(
                self.network_stats.average_slot_time.load(std::sync::atomic::Ordering::Relaxed)
            ),
            active_validators: std::sync::atomic::AtomicU64::new(
                self.network_stats.active_validators.load(std::sync::atomic::Ordering::Relaxed)
            ),
        }
    }
}

impl ValidatorTracker {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            validators: Arc::new(DashMap::new()),
            last_update: Arc::new(std::sync::RwLock::new(None)),
        })
    }

    pub async fn update_validator_info(&self) -> Result<()> {
        info!("Updating validator information");
        
        *self.last_update.write().unwrap() = Some(Instant::now());
        
        Ok(())
    }

    pub fn get_validator_count(&self) -> usize {
        self.validators.len()
    }

    pub fn get_validator(&self, pubkey: &Pubkey) -> Option<ValidatorInfo> {
        self.validators.get(pubkey).map(|entry| entry.clone())
    }
}

pub async fn health_check() -> Result<()> {
    let rpc_client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
    
    println!("Checking Solana network health...");
    
    let slot = rpc_client.get_slot()?;
    let epoch_info = rpc_client.get_epoch_info()?;
    let version = rpc_client.get_version()?;
    
    println!("✅ Network Status:");
    println!("   Current Slot: {}", slot);
    println!("   Current Epoch: {}", epoch_info.epoch);
    println!("   Slot in Epoch: {}/{}", epoch_info.slot_index, epoch_info.slots_in_epoch);
    println!("   Solana Version: {}", version.solana_core);
    
    let block_time_result = rpc_client.get_block_time(slot)?;
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;
    let lag = current_time - block_time_result;
    println!("   Block Lag: {}s", lag);
    
    println!("✅ Network is healthy and reachable");
    
    Ok(())
}