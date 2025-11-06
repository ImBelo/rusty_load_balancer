use hyper::{Response, StatusCode};
use tracing::error;

pub fn handle_proxy_error(error: anyhow::Error) -> Response<hyper::Body> {
    error!("Proxy error: {}", error);

    Response::builder()
        .status(StatusCode::BAD_GATEWAY)
        .body(hyper::Body::from(format!("Bad Gateway: {error}")))
        .unwrap()
}

pub fn no_healthy_backends() -> Response<hyper::Body> {
    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .body(hyper::Body::from("No healthy backends available"))
        .unwrap()
}

pub fn compression_failed(error: anyhow::Error) -> Response<hyper::Body> {
    error!("Compression failed: {}", error);

    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(hyper::Body::from("Internal server error: compression failed"))
        .unwrap()
}

pub fn compression_failed_graceful(error: anyhow::Error, original_response: Response<hyper::Body>) -> Response<hyper::Body> {
    error!("Compression failed, returning uncompressed response: {}", error);
    
    original_response
}

pub fn create_error_response(status: StatusCode, message: String) -> Response<hyper::Body> {
    Response::builder()
        .status(status)
        .body(hyper::Body::from(message))
        .unwrap()
}

pub fn modify_response(mut response: Response<hyper::Body>) -> Response<hyper::Body> {
    response.headers_mut().insert(
        "X-Load-Balancer",
        "rust-lb".parse().unwrap()
    );
    response
}
