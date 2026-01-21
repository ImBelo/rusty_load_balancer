pub mod algorithms;
use crate::backend::{BackendPool, HealthCheck, LoadBalancingStrategy};
use crate::proxy::ProxyHandler;
use crate::config::Config;
use hyper::service::Service;
use hyper::Server;
use std::net::SocketAddr;
use tracing::{info, error};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_rustls::rustls::{Certificate, PrivateKey, ServerConfig};
use tokio_rustls::TlsAcceptor;
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::result::Result::{Ok,Err};
use anyhow::Context; 

pub struct LoadBalancer {
    config: Config,
    backend_pool: BackendPool,
    load_balancer_url: String,
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

        let load_balancer_url = format!("http://{}:{}",config.host.clone(),config.port.clone());


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
            load_balancer_url,
        })
    }

    pub async fn start(self) -> anyhow::Result<()> {
        info!("Starting Load Balancer...");

        self.start_health_checks().await;
        let http_server = self.start_http_server();
        let https_server = self.start_https_server();
        tokio::try_join!(http_server,https_server)
            .context("Critical failure in one of the server instances")?;

        Ok(())
    }

    async fn start_health_checks(&self) {
        let health_check = HealthCheck::new(
            self.backend_pool.clone(),
            self.config.health_check_interval,
            self.load_balancer_url.clone(),
        );

        let _handle = health_check.start().await;
        info!("Health checks started with interval: {}s", self.config.health_check_interval);
    }

    async fn start_http_server(&self) -> anyhow::Result<()> {
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .expect("Invalid host/port configuration");


        let make_service = hyper::service::make_service_fn(|conn: &hyper::server::conn::AddrStream| {
            let remote_addr = conn.remote_addr();
            let backend_pool = self.backend_pool.clone();

            async move {
                Ok::<_, hyper::Error>(hyper::service::service_fn(move |mut req: hyper::Request<hyper::Body>| {
                    req.extensions_mut().insert(remote_addr);

                    let mut handler = ProxyHandler::new(backend_pool.clone());
                    handler.call(req)
                }))
            }
        });

        let server = Server::bind(&addr).serve(make_service);

        info!("Load Balancer running on http://{}", addr);
        info!("Load balancing strategy: {:?}", self.backend_pool.strategy);
        info!("Health check interval: {}s", self.config.health_check_interval);

        let backends = self.backend_pool.backends.load();
        for backend in backends.iter() {
            info!("Backend: {} -> {}", backend.name, backend.url);
        }

        match server.await {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("Server error: {}", e);
                Err(anyhow::anyhow!("Server error: {e}"))
            }
        }
    }
    async fn start_https_server(&self) -> anyhow::Result<()> {
        // 1. Configura l'indirizzo (usa una porta diversa, es: 3443)
        let https_port = self.config.port + 443; // Esempio: 3000 + 443 = 3443
        let addr: SocketAddr = format!("{}:{}", self.config.host, https_port).parse()?;

        // 2. Carica il Certificato e la Chiave Privata
        let cert_file = File::open("cert.pem")
            .context("File cert.pem non trovato. Generalo con openssl.")?;
        let key_file = File::open("key.pem")
            .context("File key.pem non trovato.")?;

        let mut cert_reader = BufReader::new(cert_file);
        let mut key_reader = BufReader::new(key_file);

        let cert_chain = certs(&mut cert_reader)?
            .into_iter()
            .map(Certificate)
            .collect();
        
        let mut keys = pkcs8_private_keys(&mut key_reader)?;
        if keys.is_empty() {
            return Err(anyhow::anyhow!("Nessuna chiave privata trovata in key.pem"));
        }
        let private_key = PrivateKey(keys.remove(0));

        // 3. Configura Rustls per il server
        let tls_config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert_chain, private_key)?;

        let acceptor = TlsAcceptor::from(Arc::new(tls_config));
        let listener = TcpListener::bind(&addr).await?;

        info!("HTTPS Server listening on https://{}", addr);

        // 4. Loop di accettazione
        loop {
            let (stream, remote_addr) = listener.accept().await?;
            let acceptor = acceptor.clone();
            let backend_pool = self.backend_pool.clone();

            tokio::spawn(async move {
                // Esegue l'handshake TLS
                match acceptor.accept(stream).await {
                    Ok(tls_stream) => {
                        // Configura il servizio Hyper sopra lo stream criptato
                        let service = hyper::service::service_fn(move |mut req| {
                            req.extensions_mut().insert(remote_addr);
                            let mut handler = ProxyHandler::new(backend_pool.clone());
                            async move { handler.call(req).await }
                        });

                        if let Err(err) = hyper::server::conn::Http::new()
                            .serve_connection(tls_stream, service)
                            .await 
                        {
                            error!("Errore nella connessione HTTPS: {:?}", err);
                        }
                    }
                    Err(e) => error!("Errore handshake TLS: {:?}", e),
                }
            });
        }
    }

    pub fn get_backend_pool(&self) -> BackendPool {
        self.backend_pool.clone()
    }
}
