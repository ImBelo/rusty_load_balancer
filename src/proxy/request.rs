use hyper::{Client, Request, Response};
use hyper_rustls::HttpsConnector;
use tracing::{info};
use anyhow::{Context, Ok, Result};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;
use crate::proxy::response::modify_response;
use hyper::client::HttpConnector;
use hyper::Uri;

type CLientType = HttpsConnector<HttpConnector>;

pub async fn forward_request(
    req: Request<hyper::Body>,
    backend: &crate::backend::server::Backend,
    client: &Client<CLientType>,
) -> Result<Response<hyper::Body>> {
    // 1. Prepariamo l'URI del backend
    let backend_uri_str = prepare_backend_uri(req.uri(), &backend.url);
    let parsed_uri: Uri = backend_uri_str.parse()
        .context("Failed to parse backend URI")?;
    // 2. Salviamo i dati necessari prima di consumare la richiesta originale
    let accept_encoding = req.headers()
        .get("accept-encoding")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let client_ip = req.extensions()
        .get::<std::net::SocketAddr>()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // 3. Scomponiamo la richiesta originale (parts contiene gli headers)
    let (mut parts, body) = req.into_parts();

    // 4. Aggiorniamo l'URI della richiesta verso il backend
    parts.uri = parsed_uri.clone();

    // 5. Fondamentale per HTTPS: Aggiorniamo l'header HOST 
    // Deve corrispondere all'host del backend, non a quello del proxy
    if let Some(host) = parsed_uri.host() {
        let host_val = if let Some(port) = parsed_uri.port() {
            format!("{}:{}", host, port)
        } else {
            host.to_string()
        };
        
        if let std::result::Result::Ok(header_val) = host_val.parse() {
            parts.headers.insert(hyper::header::HOST, header_val);
        }
    }

    // 6. USIAMO IL TUO METODO per aggiungere/aggiornare gli headers di tracing
    // Questo gestirà X-Forwarded-For, X-Real-IP, ecc.
    add_tracing_headers(&mut parts.headers, &client_ip);

    // 7. Ricostruiamo la richiesta per il backend
    let backend_req = Request::from_parts(parts, body);

    info!("Forwarding request to: {}", backend_req.uri());

    // 8. Esecuzione della chiamata al backend
    let backend_response = client.request(backend_req).await
        .context("Failed to forward request to backend")?;

    // 9. Gestione della compressione adattiva della risposta
    let compressed_response = compress_response_adaptive(backend_response, accept_encoding.as_deref())
        .await
        .context("Response compression failed")?;

    // 10. Modifiche finali (es. header di sicurezza) e ritorno
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
    // Usiamo HeaderValue::from_static o gestiamo l'errore per evitare panic
    headers.insert("X-Forwarded-By", hyper::header::HeaderValue::from_static("rust-load-balancer"));

    // Gestione X-Forwarded-For (concatenazione catena proxy)
    let new_xff = if let Some(existing) = headers.get("X-Forwarded-For") {
        if let std::result::Result::Ok(existing_str) = existing.to_str() {
            format!("{}, {}", existing_str, client_ip)
        } else {
            client_ip.to_string()
        }
    } else {
        client_ip.to_string()
    };

    if let std::result::Result::Ok(val) = new_xff.parse() {
        headers.insert("X-Forwarded-For", val);
    }

    // Imposta il protocollo (se non presente)
    headers.entry("X-Forwarded-Proto").or_insert_with(|| "http".parse().unwrap());

    // Imposta l'IP reale (se non presente)
    headers.entry("X-Real-IP").or_insert_with(|| client_ip.parse().unwrap());
}

