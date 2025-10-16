pub mod algorithms;
use crate::backend::{BackendPool, HealthCheck, LoadBalancingStrategy};
use crate::proxy::ProxyHandler;
use crate::config::Config;
use hyper::Server;
use std::net::SocketAddr;
use tracing::{info, error};

pub struct LoadBalancer {
    config: Config,
    backend_pool: BackendPool,
}

impl LoadBalancer {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        info!("Initializing Load Balancer with config: {:?}", config);
        
        let backends = config.backends.iter()
            .map(|backend_config| {
                crate::backend::server::Backend::new(
                    backend_config.url.clone(),
                    backend_config.name.clone(),
                    backend_config.weight.unwrap_or(1),
                )
            })
            .collect();

        let strategy = match config.lb_strategy.as_str() {
            "round_robin" => LoadBalancingStrategy::RoundRobin,
            "random" => LoadBalancingStrategy::Random,
            "least_connections" => LoadBalancingStrategy::LeastConnections,
            "weighted_round_robin" => LoadBalancingStrategy::WeightedRoundRobin,
            _ => LoadBalancingStrategy::RoundRobin,
        };

        let backend_pool = BackendPool::new(backends, strategy);

        Ok(Self {
            config,
            backend_pool,
        })
    }

    pub async fn start(self) -> anyhow::Result<()> {
        info!("Starting Load Balancer...");

        self.start_health_checks().await;
        self.start_http_server().await?;

        Ok(())
    }

    async fn start_health_checks(&self) {
        let health_check = HealthCheck::new(
            self.backend_pool.clone(),
            self.config.health_check_interval,
        );

        let _handle = health_check.start().await;
        info!("Health checks started with interval: {}s", self.config.health_check_interval);
    }

    async fn start_http_server(&self) -> anyhow::Result<()> {
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .expect("Invalid host/port configuration");

        let proxy_handler = ProxyHandler::new(self.backend_pool.clone());

        // Usa make_service per creare un service per ogni connessione
        let make_service = hyper::service::make_service_fn(|_conn| {
            let handler = proxy_handler.clone();
            async move {
                Ok::<_, hyper::Error>(handler)
            }
        });

        let server = Server::bind(&addr).serve(make_service);

        info!("🚀 Load Balancer running on http://{}", addr);
        info!("📊 Load balancing strategy: {:?}", self.backend_pool.strategy);
        info!("🔍 Health check interval: {}s", self.config.health_check_interval);

        let backends = self.backend_pool.backends.load();
        for backend in backends.iter() {
            info!("🎯 Backend: {} -> {}", backend.name, backend.url);
        }

        match server.await {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("Server error: {}", e);
                Err(anyhow::anyhow!("Server error: {}", e))
            }
        }
    }

    pub fn get_backend_pool(&self) -> BackendPool {
        self.backend_pool.clone()
    }
}
