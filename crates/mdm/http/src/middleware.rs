//! HTTP middleware.

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

/// Logging middleware.
pub async fn logging_middleware(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();

    tracing::info!(method = %method, uri = %uri, "incoming request");

    let response = next.run(request).await;

    tracing::info!(
        method = %method,
        uri = %uri,
        status = %response.status(),
        "request completed"
    );

    response
}
