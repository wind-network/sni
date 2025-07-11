use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, error};

use crate::config::ApiConfig;
use crate::storage::StorageManager;

#[derive(Debug, Clone)]
pub struct ApiServer {
    config: ApiConfig,
    storage: Arc<StorageManager>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub blocks_indexed: u64,
    pub transactions_indexed: u64,
}

impl ApiServer {
    pub fn new(config: ApiConfig, storage: Arc<StorageManager>) -> Self {
        Self { config, storage }
    }

    pub async fn start(&self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr).await?;
        
        info!("SNI API server listening on {}", addr);
        info!("GraphQL Playground: http://{}/playground", addr);
        info!("Health endpoint: http://{}/health", addr);
        
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    info!("New connection from {}", addr);
                    let storage = self.storage.clone();
                    
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(stream, storage).await {
                            error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    async fn handle_connection(
        mut stream: tokio::net::TcpStream,
        storage: Arc<StorageManager>,
    ) -> Result<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        
        let mut buffer = [0; 1024];
        let n = stream.read(&mut buffer).await?;
        let request = String::from_utf8_lossy(&buffer[..n]);
        
        let response = if request.contains("GET /health") {
            Self::handle_health(storage).await
        } else if request.contains("GET /playground") {
            Self::handle_playground().await
        } else {
            Self::handle_not_found().await
        };
        
        stream.write_all(response.as_bytes()).await?;
        stream.flush().await?;
        
        Ok(())
    }

    async fn handle_health(storage: Arc<StorageManager>) -> String {
        let health_data = match Self::get_health_data(storage).await {
            Ok(data) => ApiResponse {
                success: true,
                data: Some(data),
                error: None,
            },
            Err(e) => ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            },
        };

        let json = serde_json::to_string_pretty(&health_data)
            .unwrap_or_else(|_| "{}".to_string());

        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            json.len(),
            json
        )
    }

    async fn get_health_data(storage: Arc<StorageManager>) -> Result<HealthResponse> {
        let blocks_indexed = storage.get_block_count().await?;
        let transactions_indexed = storage.get_transaction_count().await?;

        Ok(HealthResponse {
            status: "healthy".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: 0, // TODO: Calculate actual uptime
            blocks_indexed,
            transactions_indexed,
        })
    }

    async fn handle_playground() -> String {
        let html = r#"
<!DOCTYPE html>
<html>
<head>
    <title>SNI GraphQL Playground</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .container { max-width: 800px; margin: 0 auto; }
        .hero { text-align: center; margin-bottom: 40px; }
        .api-info { background: #f5f5f5; padding: 20px; border-radius: 8px; }
        .endpoint { margin: 10px 0; font-family: monospace; }
    </style>
</head>
<body>
    <div class="container">
        <div class="hero">
            <h1>🌊 SNI (Solana Network Indexer)</h1>
            <p>Ultra-fast Solana blockchain indexer powered by Tide engine</p>
        </div>
        
        <div class="api-info">
            <h2>Available Endpoints</h2>
            <div class="endpoint">GET /health - Health check and statistics</div>
            <div class="endpoint">GET /playground - This page</div>
            
            <h3>Coming Soon</h3>
            <div class="endpoint">POST /graphql - GraphQL endpoint</div>
            <div class="endpoint">WS /subscriptions - Real-time subscriptions</div>
        </div>
    </div>
</body>
</html>
        "#;

        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
            html.len(),
            html
        )
    }

    async fn handle_not_found() -> String {
        let response = "404 Not Found";
        format!(
            "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
            response.len(),
            response
        )
    }
}