//! REST API handlers.

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use color_eyre::eyre::WrapErr as _;
use serde::{Deserialize, Serialize};

use mdm_storage::{CommandStore, PushCertStore};

/// Push certificate response.
#[derive(Debug, Serialize, Deserialize)]
pub struct PushCertResponse {
    pub topic: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_after: Option<String>,
}

/// Store a push certificate.
pub async fn store_push_cert<S>(State(store): State<S>, body: Bytes) -> impl IntoResponse
where
    S: PushCertStore,
{
    match store_push_cert_inner(&store, &body) {
        Ok(response) => (StatusCode::OK, Json(response)),
        Err(e) => {
            tracing::error!(error = %e, "failed to store push cert");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(PushCertResponse {
                    topic: String::new(),
                    not_after: None,
                }),
            )
        }
    }
}

fn store_push_cert_inner<S: PushCertStore>(
    store: &S,
    body: &[u8],
) -> color_eyre::eyre::Result<PushCertResponse> {
    let body_str = std::str::from_utf8(body).wrap_err("body is not valid UTF-8")?;

    // Body should be PEM cert + key concatenated
    let parts: Vec<&str> = body_str.split("-----END CERTIFICATE-----").collect();
    if parts.len() < 2 {
        color_eyre::eyre::bail!("body should contain cert and key PEM");
    }

    let cert_pem = format!("{}-----END CERTIFICATE-----", parts[0]);
    let key_pem = parts[1].trim().to_string();

    // Extract topic from cert
    let cert_der = mdm_crypto::parse_pem_cert(&cert_pem)?;
    let topic = mdm_crypto::extract_topic_from_cert(&cert_der)?;

    store.store_push_cert(&topic, &cert_pem, &key_pem)?;

    Ok(PushCertResponse {
        topic,
        not_after: None,
    })
}

/// Get push certificate info.
pub async fn get_push_cert<S>(State(store): State<S>) -> impl IntoResponse
where
    S: PushCertStore,
{
    // This would need a topic parameter in practice
    (StatusCode::NOT_IMPLEMENTED, "not implemented")
}

/// Push notifications to devices.
pub async fn push_handler(Path(ids): Path<String>) -> impl IntoResponse {
    // TODO: Implement push
    tracing::info!(ids = %ids, "push requested");
    StatusCode::NOT_IMPLEMENTED
}

/// Enqueue command request.
#[derive(Debug, Deserialize)]
pub struct EnqueueRequest {
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub no_push: bool,
}

/// Enqueue command response.
#[derive(Debug, Serialize)]
pub struct EnqueueResponse {
    pub command_uuid: String,
    pub request_type: String,
}

/// Enqueue a command for devices.
pub async fn enqueue_handler<S>(
    State(store): State<S>,
    Path(ids): Path<String>,
    body: Bytes,
) -> impl IntoResponse
where
    S: CommandStore,
{
    match enqueue_inner(&store, &ids, &body) {
        Ok(response) => (StatusCode::OK, Json(response)),
        Err(e) => {
            tracing::error!(error = %e, "failed to enqueue command");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(EnqueueResponse {
                    command_uuid: String::new(),
                    request_type: String::new(),
                }),
            )
        }
    }
}

fn enqueue_inner<S: CommandStore>(
    store: &S,
    ids: &str,
    body: &[u8],
) -> color_eyre::eyre::Result<EnqueueResponse> {
    // Parse command from body (raw plist)
    let cmd: mdm_core::Command =
        plist::from_bytes(body).wrap_err("failed to parse command plist")?;

    // Enqueue for each ID
    for id_str in ids.split(',') {
        let id = mdm_core::EnrollId {
            enroll_type: mdm_core::EnrollType::Device,
            id: id_str.trim().to_string(),
            parent_id: None,
        };

        store
            .enqueue_command(&id, body)
            .wrap_err_with(|| format!("failed to enqueue for {}", id_str))?;
    }

    Ok(EnqueueResponse {
        command_uuid: cmd.command_uuid,
        request_type: cmd.command.request_type,
    })
}
