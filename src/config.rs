use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SniConfig {
    pub network: NetworkConfig,
    pub storage: StorageConfig,
    pub api: ApiConfig,
    pub indexing: IndexingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub rpc_url: String,
    pub websocket_url: String,
    pub commitment: String,
    pub auto_discover_validators: bool,
    pub max_validator_connections: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub database_url: String,
    pub enable_compression: bool,
    pub batch_size: usize,
    pub flush_interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
    pub enable_graphql: bool,
    pub enable_websockets: bool,
    pub cors_origins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
    pub index_accounts: bool,
    pub index_transactions: bool,
    pub index_blocks: bool,
    pub track_validators: bool,
    pub track_network_health: bool,
    pub program_filters: Vec<String>,
}

impl SniConfig {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        if !path.as_ref().exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)?;
        let config: SniConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

impl Default for SniConfig {
    fn default() -> Self {
        Self {
            network: NetworkConfig {
                rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
                websocket_url: "wss://api.mainnet-beta.solana.com".to_string(),
                commitment: "confirmed".to_string(),
                auto_discover_validators: true,
                max_validator_connections: 5,
            },
            storage: StorageConfig {
                database_url: "sqlite:sni.db".to_string(),
                enable_compression: true,
                batch_size: 1000,
                flush_interval_ms: 5000,
            },
            api: ApiConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                enable_graphql: true,
                enable_websockets: true,
                cors_origins: vec!["*".to_string()],
            },
            indexing: IndexingConfig {
                index_accounts: true,
                index_transactions: true,
                index_blocks: true,
                track_validators: true,
                track_network_health: true,
                program_filters: vec![],
            },
        }
    }
}