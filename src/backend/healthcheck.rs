use super::pool::BackendPool;
use super::server::BackendStatus;
use reqwest::Client;
use std::time::Duration;
use tracing::{info, warn};
use crate::backend::Backend;

pub struct HealthCheck {
    pool: BackendPool,
    interval_secs: u64,
    client: Client,
}

impl HealthCheck {
    pub fn new(pool: BackendPool, interval_secs: u64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            pool,
            interval_secs,
            client,
        }
    }

    pub async fn start(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            self.run().await;
        })
    }

    async fn run(self) {
        let interval = Duration::from_secs(self.interval_secs);
        loop {
            self.check_all_backends().await;
            tokio::time::sleep(interval).await;
        }
    }

    async fn check_all_backends(&self) {
        let backends = self.pool.backends.load();

        let mut handles = Vec::new();

        // Spawn all health check tasks
        for (index, backend) in backends.iter().enumerate() {
            let client = self.client.clone();
            let backend = backend.clone();

            let handle = tokio::spawn(async move {
                let status = Self::check_single_backend(&client, &backend).await;
                (index, backend, status)
            });

            handles.push(handle);
        }
    
        for handle in handles {
            let pool = self.pool.clone();
            match handle.await {
                Ok((index, backend, status)) => {
                    pool.update_backend_status(index, status).await;

                    match status {
                        BackendStatus::Healthy => info!("Backend {} ({}) is healthy", backend.name, backend.url),
                        BackendStatus::Unhealthy => warn!("Backend {} ({}) is unhealthy", backend.name, backend.url),
                        BackendStatus::Unknown => warn!("Backend {} ({}) status unknown", backend.name, backend.url),
                    }
                }
                Err(e) => {
                    warn!("Health check task failed: {}", e);
                }
            }
        }
    }

    // Helper function that doesn't borrow self
    async fn check_single_backend(client: &Client, backend: &Backend) -> BackendStatus {
        let health_url = format!("{}/", backend.url);

        match client.get(&health_url).send().await {
            Ok(response) if response.status().is_success() => BackendStatus::Healthy,
            Ok(_) => BackendStatus::Unhealthy,
            Err(_) => BackendStatus::Unhealthy,
        }
    }
    
}
