use anyhow::Result;
use sqlx::{SqlitePool, Row};
use serde::{Serialize, Deserialize};
use std::path::Path;
use tracing::info;

use crate::config::StorageConfig;

#[derive(Debug, Clone)]
pub struct StorageManager {
    pool: SqlitePool,
    config: StorageConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexedData {
    Block {
        slot: u64,
        parent_slot: u64,
        height: u64,
        timestamp: i64,
        blockhash: String,
        transactions_count: usize,
    },
    Transaction {
        signature: String,
        slot: u64,
        timestamp: i64,
        success: bool,
        transaction_data: Vec<u8>,
    },
    Account {
        pubkey: String,
        owner: String,
        lamports: u64,
        slot: u64,
        executable: bool,
        rent_epoch: u64,
        data_hash: String,
    },
    Slot {
        slot: u64,
        parent: Option<u64>,
        status: String,
        timestamp: i64,
    },
}

impl StorageManager {
    pub async fn new(config: &StorageConfig) -> Result<Self> {
        let pool = if config.database_url.starts_with("sqlite:") {
            let db_path = config.database_url.strip_prefix("sqlite:").unwrap();
            
            if !Path::new(db_path).exists() {
                info!("Creating new SQLite database at {}", db_path);
            }
            
            SqlitePool::connect(&config.database_url).await?
        } else {
            return Err(anyhow::anyhow!("Only SQLite is supported in this basic implementation"));
        };

        let storage = Self {
            pool,
            config: config.clone(),
        };

        storage.initialize_schema().await?;
        
        Ok(storage)
    }

    async fn initialize_schema(&self) -> Result<()> {
        info!("Initializing database schema");

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS blocks (
                slot INTEGER PRIMARY KEY,
                parent_slot INTEGER NOT NULL,
                height INTEGER NOT NULL,
                timestamp INTEGER NOT NULL,
                blockhash TEXT NOT NULL,
                transactions_count INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS transactions (
                signature TEXT PRIMARY KEY,
                slot INTEGER NOT NULL,
                timestamp INTEGER NOT NULL,
                success BOOLEAN NOT NULL,
                transaction_data BLOB NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS accounts (
                pubkey TEXT PRIMARY KEY,
                owner TEXT NOT NULL,
                lamports INTEGER NOT NULL,
                slot INTEGER NOT NULL,
                executable BOOLEAN NOT NULL,
                rent_epoch INTEGER NOT NULL,
                data_hash TEXT NOT NULL,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS slots (
                slot INTEGER PRIMARY KEY,
                parent INTEGER,
                status TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_blocks_timestamp ON blocks(timestamp)")
            .execute(&self.pool)
            .await?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_transactions_slot ON transactions(slot)")
            .execute(&self.pool)
            .await?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_accounts_owner ON accounts(owner)")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn store(&self, data: IndexedData) -> Result<()> {
        match data {
            IndexedData::Block { slot, parent_slot, height, timestamp, blockhash, transactions_count } => {
                sqlx::query(
                    "INSERT OR REPLACE INTO blocks (slot, parent_slot, height, timestamp, blockhash, transactions_count) VALUES (?, ?, ?, ?, ?, ?)"
                )
                .bind(slot as i64)
                .bind(parent_slot as i64)
                .bind(height as i64)
                .bind(timestamp)
                .bind(blockhash)
                .bind(transactions_count as i64)
                .execute(&self.pool)
                .await?;
            }
            IndexedData::Transaction { signature, slot, timestamp, success, transaction_data } => {
                sqlx::query(
                    "INSERT OR REPLACE INTO transactions (signature, slot, timestamp, success, transaction_data) VALUES (?, ?, ?, ?, ?)"
                )
                .bind(signature)
                .bind(slot as i64)
                .bind(timestamp)
                .bind(success)
                .bind(transaction_data)
                .execute(&self.pool)
                .await?;
            }
            IndexedData::Account { pubkey, owner, lamports, slot, executable, rent_epoch, data_hash } => {
                sqlx::query(
                    "INSERT OR REPLACE INTO accounts (pubkey, owner, lamports, slot, executable, rent_epoch, data_hash) VALUES (?, ?, ?, ?, ?, ?, ?)"
                )
                .bind(pubkey)
                .bind(owner)
                .bind(lamports as i64)
                .bind(slot as i64)
                .bind(executable)
                .bind(rent_epoch as i64)
                .bind(data_hash)
                .execute(&self.pool)
                .await?;
            }
            IndexedData::Slot { slot, parent, status, timestamp } => {
                sqlx::query(
                    "INSERT OR REPLACE INTO slots (slot, parent, status, timestamp) VALUES (?, ?, ?, ?)"
                )
                .bind(slot as i64)
                .bind(parent.map(|p| p as i64))
                .bind(status)
                .bind(timestamp)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn get_latest_slot(&self) -> Result<Option<u64>> {
        let row = sqlx::query("SELECT MAX(slot) as max_slot FROM blocks")
            .fetch_one(&self.pool)
            .await?;
        
        let slot: Option<i64> = row.try_get("max_slot")?;
        Ok(slot.map(|s| s as u64))
    }

    pub async fn get_block_count(&self) -> Result<u64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM blocks")
            .fetch_one(&self.pool)
            .await?;
        
        let count: i64 = row.try_get("count")?;
        Ok(count as u64)
    }

    pub async fn get_transaction_count(&self) -> Result<u64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM transactions")
            .fetch_one(&self.pool)
            .await?;
        
        let count: i64 = row.try_get("count")?;
        Ok(count as u64)
    }
}