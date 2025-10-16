use crate::backend::pool::BackendPool;
use crate::proxy::request::forward_request;
use crate::proxy::response::{handle_proxy_error, no_healthy_backends, modify_response};
use hyper::{Request, Response};
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tracing::{info, error};

#[derive(Clone)]
pub struct ProxyHandler {
    pub backend_pool: BackendPool,
}

impl ProxyHandler {
    pub fn new(backend_pool: BackendPool) -> Self {
        Self { backend_pool }
    }

    pub async fn handle_request(&self, req: Request<hyper::Body>) -> Result<Response<hyper::Body>, Infallible> {
        info!("Incoming request: {} {}", req.method(), req.uri());

        let backend = match self.backend_pool.select_backend().await {
            Some(backend) => backend,
            None => {
                error!("No healthy backends available");
                return Ok(no_healthy_backends());
            }
        };

        self.backend_pool.increment_connections(&backend).await;

        info!("Selected backend: {}", backend.url);

        match forward_request(req, &backend).await {
            Ok(resp) => Ok(modify_response(resp)),
            Err(e) => {
                self.backend_pool.decrement_connections(&backend).await;
                Ok(handle_proxy_error(e))
            }
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
