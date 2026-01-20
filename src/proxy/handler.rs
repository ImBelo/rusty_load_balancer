use crate::backend::pool::BackendPool;
use crate::proxy::request::forward_request;
use crate::proxy::response::{handle_proxy_error, no_healthy_backends};
use hyper::{Request, Response,StatusCode};
use std::time::Duration;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use hyper::Client;
use tracing::{info, error};
use crate::backend::Backend;
use tokio::sync::Semaphore;
use std::sync::Arc;
use hyper_rustls::HttpsConnector;
use hyper::client::HttpConnector;

// Definiamo un tipo per chiarezza
type ClientType = Client<HttpsConnector<HttpConnector>, hyper::Body>;

#[derive(Clone)]
pub struct ProxyHandler {
    pub backend_pool: BackendPool,
    pub http_client: ClientType,
    pub concurrency_limiter: Arc<Semaphore>, 
}

impl ProxyHandler {
    pub fn new(backend_pool: BackendPool) -> Self {
        let https = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .https_or_http()
            .enable_http1()
            .build();
        // 2. Crea il client con il connettore HTTPS
        let http_client = Client::builder()
            .pool_idle_timeout(Duration::from_secs(30))
            .build(https);

        Self { 
            backend_pool,
            http_client,
            concurrency_limiter: Arc::new(Semaphore::new(100)), 
        }
    }

    pub async fn handle_request(&self, req: Request<hyper::Body>) -> Result<Response<hyper::Body>, Infallible> {
        // solo 100 permessi
        let _permit = self.concurrency_limiter.acquire().await.unwrap();
        // Se é una richiesta di healthcheck
        if req.uri().path().starts_with("/health/") {
            return self.handle_health_check(req).await;
        }
        // Richiesta normale
        info!("Incoming request: {} {}", req.method(), req.uri());

        // Prendi il backend e incrementa le connessioni nel pool
        let backend = match self.backend_pool.select_and_increment().await {
            Some(backend) => {
                info!("Selected backend: {}", backend.url);
               // println!("INCREMENTED: {} (now: {})", backend.url, self.backend_pool.get_connection_count(&backend).await);
                backend
            },
            None => {
                error!("No healthy backends available");
                return Ok(no_healthy_backends());
            }
        };
        // Hardcoded backend-1 1 secondo di risposta per testare algoritmo di least-connection
        /*if backend.url ==  "http://127.0.0.1:8081" {
            backend.simulate_delay().await;
        }*/
        // Fai il forward della richiesta e aggiungi header e in caso compremi
        let forward = match forward_request(req, &backend, &self.http_client).await {
            Ok(resp) => resp,
            Err(e) => handle_proxy_error(e)
        };
        // Dopo il forward decrementa le connessioni nel pool
        self.backend_pool.decrement_connections(&backend).await;
       // println!("DECREMENTED: {} (now: {})", backend.url, self.backend_pool.get_connection_count(&backend).await);

        Ok(forward)

    }

    async fn handle_health_check(&self, req: Request<hyper::Body>) -> Result<Response<hyper::Body>, Infallible> {
        // Prendi il nome del backend
        let backend_name = req.uri().path().trim_start_matches("/health/");

        if let Some(backend) = self.backend_pool.get_backend_by_name(backend_name).await {
            // Richiesta diretta per vedere se é healthy
            let is_healthy = self.direct_health_check(&backend).await;

            let status = if is_healthy { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
            return Ok(Response::builder().status(status).body(hyper::Body::from("")).unwrap());
        }

        Ok(Response::builder().status(StatusCode::NOT_FOUND).body(hyper::Body::from("")).unwrap())
    }

    async fn direct_health_check(&self, backend: &Backend) -> bool {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();
        match client.get(&backend.url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}

impl hyper::service::Service<Request<hyper::Body>> for ProxyHandler {
    type Response = Response<hyper::Body>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<hyper::Body>) -> Self::Future {
        let handler = self.clone();
        
        Box::pin(async move {
            handler.handle_request(req).await
        })
    }
}
