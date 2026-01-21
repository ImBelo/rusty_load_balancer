use super::pool::BackendPool;
use super::server::BackendStatus;
use reqwest::Client;
use tokio::join;
use std::{result, time::Duration};
use tracing::{info, warn};
use crate::backend::Backend;

pub struct HealthCheck {
    pool: BackendPool,
    interval_secs: u64,
    client: Client,
    load_balancer_url: String,
}

impl HealthCheck {
    pub fn new(pool: BackendPool, interval_secs: u64, load_balancer_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            pool,
            interval_secs,
            client,
            load_balancer_url,
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

        for (index, backend) in backends.iter().enumerate() {
            let client = self.client.clone();
            let backend = backend.clone();
            let url = self.load_balancer_url.clone();
            let pool = self.pool.clone();

            tokio::spawn(async move {
                let status = Self::check_single_backend(&client, &backend, url).await;
                // Update status and log
                pool.update_backend_status(index, status).await;
                match status {
                    BackendStatus::Healthy => info!("Backend {} ({}) is healthy", backend.name, backend.url),
                    BackendStatus::Unhealthy => warn!("Backend {} ({}) is unhealthy", backend.name, backend.url),
                    BackendStatus::Unknown => warn!("Backend {} ({}) status unknown", backend.name, backend.url),
                }
            });

        }
    }

    async fn check_single_backend(client: &Client, backend: &Backend,load_balancer_url: String) -> BackendStatus {
        let health_url = format!("{}/health/{}",load_balancer_url, backend.name);

        match client.get(&health_url).send().await {
            Ok(response) if response.status().is_success() => BackendStatus::Healthy,
            Ok(_) => BackendStatus::Unhealthy,
            Err(_) => BackendStatus::Unhealthy,
        }
    }
    
}
