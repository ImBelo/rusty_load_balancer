use hyper::{Client, Request, Response};
use tracing::{info};
use anyhow::{Result, Context};

pub async fn forward_request(
    req: Request<hyper::Body>,
    backend: &crate::backend::server::Backend,
) -> Result<Response<hyper::Body>> {
    // Prepara l'URI per il backend
    let backend_uri = prepare_backend_uri(req.uri(), &backend.url);
    
    // Scomponi la request originale
    let (mut parts, body) = req.into_parts();
    
    // Aggiorna l'URI
    parts.uri = backend_uri.parse()
        .context("Failed to parse backend URI")?;

    // Aggiungi headers di tracing
    add_tracing_headers(&mut parts.headers);
    
    // Ricostruisci la request
    let backend_req = Request::from_parts(parts, body);

    info!("Forwarding request to backend: {} {}", backend_req.method(), backend_req.uri());

    // Crea client HTTP
    let client = Client::new();

    // Inoltra la request e attendi risposta
    client.request(backend_req).await
        .context("Failed to forward request to backend")
}

fn prepare_backend_uri(original_uri: &hyper::Uri, backend_url: &str) -> String {
    let path_and_query = original_uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");

    // Rimuovi slash duplicati
    let backend_url = backend_url.trim_end_matches('/');
    let path_and_query = path_and_query.trim_start_matches('/');
    
    if path_and_query.is_empty() {
        backend_url.to_string()
    } else {
        format!("{backend_url}/{path_and_query}")
    }
}

fn add_tracing_headers(headers: &mut hyper::HeaderMap) {
    headers.insert("X-Forwarded-By", "rust-load-balancer".parse().unwrap());
    
    if !headers.contains_key("X-Forwarded-For") {
        headers.insert("X-Forwarded-For", "unknown".parse().unwrap());
    }

    headers.insert("X-Forwarded-Proto", "http".parse().unwrap());
}
