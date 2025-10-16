use super::pool::BackendPool;
use super::server::BackendStatus;
use reqwest::Client;
use std::time::Duration;
use tracing::{info, warn};

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
        
        for (index, backend) in backends.iter().enumerate() {
            let status = self.check_backend(backend).await;
            self.pool.update_backend_status(index, status).await;

            match status {
                BackendStatus::Healthy => info!("Backend {} ({}) is healthy", backend.name, backend.url),
                BackendStatus::Unhealthy => warn!("Backend {} ({}) is unhealthy", backend.name, backend.url),
                BackendStatus::Unknown => warn!("Backend {} ({}) status unknown", backend.name, backend.url),
            }
        }
    }

    async fn check_backend(&self, backend: &super::server::Backend) -> BackendStatus {
        let health_url = format!("{}/", backend.url);
        
        match self.client.get(&health_url).send().await {
            Ok(response) if response.status().is_success() => BackendStatus::Healthy,
            Ok(_) => BackendStatus::Unhealthy,
            Err(_) => BackendStatus::Unhealthy,
        }
    }
}
