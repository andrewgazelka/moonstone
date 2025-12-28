//! MDM HTTP Layer
//!
//! Axum handlers for MDM check-in and command endpoints.

mod api;
mod handlers;
mod middleware;

pub use api::*;
pub use handlers::*;
pub use middleware::*;

use axum::Router;

/// Create the MDM router.
pub fn mdm_router<S>(service: S) -> Router
where
    S: mdm_service::CheckinAndCommand + Clone + 'static,
{
    use axum::routing::post;

    Router::new()
        .route("/mdm/checkin", post(handlers::checkin_handler::<S>))
        .route("/mdm/command", post(handlers::command_handler::<S>))
        .with_state(service)
}

/// Create the API router.
pub fn api_router<St>(store: St) -> Router
where
    St: mdm_storage::AllStorage + Clone + 'static,
{
    use axum::routing::{get, post, put};

    Router::new()
        .route("/v1/pushcert", put(api::store_push_cert::<St>))
        .route("/v1/pushcert", get(api::get_push_cert::<St>))
        .route("/v1/push/:ids", post(api::push_handler))
        .route("/v1/enqueue/:ids", post(api::enqueue_handler::<St>))
        .with_state(store)
}
