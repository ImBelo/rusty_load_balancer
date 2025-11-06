use hyper::{Client, Request, Response};
use tracing::{info};
use anyhow::{Context, Ok, Result};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;
use crate::proxy::response::modify_response;

pub async fn forward_request(
    req: Request<hyper::Body>,
    backend: &crate::backend::server::Backend,
    client: &Client<hyper::client::HttpConnector>,
) -> Result<Response<hyper::Body>> {
    let backend_uri = prepare_backend_uri(req.uri(), &backend.url);

    let accept_encoding = req.headers()
        .get("accept-encoding")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let client_ip = req.extensions()
        .get::<std::net::SocketAddr>()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Scomponi la request originale
    let (mut parts, body) = req.into_parts();

    // Aggiorna l'URI
    parts.uri = backend_uri.parse()
        .context("Failed to parse backend URI")?;

    add_tracing_headers(&mut parts.headers, &client_ip);
    
    // Ricostruisci la request
    let backend_req = Request::from_parts(parts, body);

    info!("Forwarding request to backend: {} {}", backend_req.method(), backend_req.uri());

    // Inoltra la request e attendi risposta
    let backend_response = client.request(backend_req).await
        .context("Failed to forward request to backend")?;

    let compressed_response = compress_response_adaptive(backend_response, accept_encoding.as_deref())
        .await
        .context("Response compression failed")?;

    Ok(modify_response(compressed_response))

}

async fn compress_response_adaptive(
    response: Response<hyper::Body>,
    accept_encoding: Option<&str>,
) -> Result<Response<hyper::Body>> {
    
    let content_type = response.headers()
        .get("content-type")
        .and_then(|ct| ct.to_str().ok())
        .unwrap_or("unknown");

    info!("Compression check - Content-Type: {}, Accept-Encoding: {:?}", 
          content_type, accept_encoding);
    // 1. Controlla se il content-type è comprimibile
    if !should_compress(&response) {
        return Ok(response);
    }
    
    // 2. Scegli algoritmo di compressione
    let algorithm = choose_compression_algorithm(accept_encoding);
    
    match algorithm {
        Some("gzip") => compress_gzip(response).await,
        Some("identity") => Ok(response), // Explicit no compression
        _ => Ok(response),  // Nessun algoritmo supportato
    }
}

fn choose_compression_algorithm(accept_encoding: Option<&str>) -> Option<&'static str> {
    accept_encoding.and_then(|ae| {
        if ae.contains("gzip") {
            Some("gzip")
        }
        else if ae.contains("identity") {
            Some("identity")

        } else {
            None
        }
    })
}

fn should_compress(response: &Response<hyper::Body>) -> bool {

    if response.headers().contains_key("content-encoding") {
        return false;
    }
    
    if let Some(content_type) = response.headers().get("content-type") {
        let ct = content_type.to_str().unwrap_or("");
        !(ct.starts_with("image/") || 
          ct.starts_with("video/") || 
          ct.starts_with("audio/") ||
          ct.contains("octet-stream") ||
          ct.contains("compressed") ||
          ct.contains("zip"))
    } else {
        true
    }
}

async fn compress_gzip(response: Response<hyper::Body>) -> Result<Response<hyper::Body>> {
    let (mut parts, body) = response.into_parts();
    let body_bytes = hyper::body::to_bytes(body)
        .await
        .context("Failed to read body for gzip compression")?;

     if body_bytes.len() < 150 {  
        return Ok(Response::from_parts(parts, hyper::Body::from(body_bytes)));
    }
    
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&body_bytes)
        .context("gzip compression failed")?;
    
    let compressed = encoder.finish()?;
    
    if compressed.len() >= body_bytes.len() {
        return Ok(Response::from_parts(parts, hyper::Body::from(body_bytes)));
    }

    parts.headers.insert(
        "content-encoding", 
        "gzip".parse().unwrap()
    );
    parts.headers.remove("content-length");

    Ok(Response::from_parts(parts, hyper::Body::from(compressed)))
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

fn add_tracing_headers(headers: &mut hyper::HeaderMap, client_ip: &str) {
    headers.insert("X-Forwarded-By", "rust-load-balancer".parse().unwrap());
    
    match headers.get("X-Forwarded-For") {
        Some(existing) => {
            let mut new_chain = existing.to_str().unwrap_or("").to_string();
            if !new_chain.is_empty() {
                new_chain.push_str(", ");
            }
            new_chain.push_str(client_ip);
            headers.insert("X-Forwarded-For", new_chain.parse().unwrap());
        }
        None => {
            headers.insert("X-Forwarded-For", client_ip.parse().unwrap());
        }
    }
    
    if !headers.contains_key("X-Forwarded-Proto") {
        headers.insert("X-Forwarded-Proto", "http".parse().unwrap());
    }
    
    if !headers.contains_key("X-Real-IP") {
        headers.insert("X-Real-IP", client_ip.parse().unwrap());
    }
}

