//! Focus-specific API endpoints.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use focus_agent::policy::FocusPolicy;
use mdm_storage::CommandStore;

/// Create the focus API router.
pub fn focus_router<S>(store: S) -> Router
where
    S: CommandStore + Clone + 'static,
{
    Router::new()
        .route("/api/focus/policy/:device_id", post(set_policy::<S>))
        .route("/api/focus/policy/:device_id", get(get_policy))
        .route("/api/focus/status/:device_id", get(get_status))
        .with_state(store)
}

/// Set focus policy request.
#[derive(Debug, Deserialize)]
pub struct SetPolicyRequest {
    pub policy: FocusPolicy,
}

/// Set focus policy response.
#[derive(Debug, Serialize)]
pub struct SetPolicyResponse {
    pub command_uuid: String,
}

/// Set a focus policy for a device.
pub async fn set_policy<S>(
    State(store): State<S>,
    Path(device_id): Path<String>,
    Json(request): Json<SetPolicyRequest>,
) -> impl IntoResponse
where
    S: CommandStore,
{
    tracing::info!(device_id = %device_id, "setting focus policy");

    // Create MDM command with focus policy
    let mut command = mdm_core::new_command("FocusPolicy");

    // Serialize policy to plist value and add to command data
    match mdm_core::to_plist_value(&request.policy) {
        Ok(value) => {
            command.command.data.insert("Policy".to_string(), value);
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to serialize policy");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SetPolicyResponse {
                    command_uuid: String::new(),
                }),
            );
        }
    }

    // Serialize command
    let command_bytes = match mdm_core::serialize_command(&command) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::error!(error = %e, "failed to serialize command");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SetPolicyResponse {
                    command_uuid: String::new(),
                }),
            );
        }
    };

    // Enqueue command
    let enroll_id = mdm_core::EnrollId {
        enroll_type: mdm_core::EnrollType::Device,
        id: device_id,
        parent_id: None,
    };

    match store.enqueue_command(&enroll_id, &command_bytes) {
        Ok(uuid) => (
            StatusCode::OK,
            Json(SetPolicyResponse { command_uuid: uuid }),
        ),
        Err(e) => {
            tracing::error!(error = %e, "failed to enqueue command");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SetPolicyResponse {
                    command_uuid: String::new(),
                }),
            )
        }
    }
}

/// Get current policy for a device.
pub async fn get_policy(Path(device_id): Path<String>) -> impl IntoResponse {
    // TODO: Store and retrieve policies
    tracing::info!(device_id = %device_id, "getting focus policy");
    StatusCode::NOT_IMPLEMENTED
}

/// Device compliance status.
#[derive(Debug, Serialize)]
pub struct DeviceStatus {
    pub device_id: String,
    pub compliant: bool,
    pub last_seen: Option<String>,
    pub policy_applied: bool,
}

/// Get compliance status for a device.
pub async fn get_status(Path(device_id): Path<String>) -> impl IntoResponse {
    tracing::info!(device_id = %device_id, "getting device status");

    // TODO: Track device compliance
    Json(DeviceStatus {
        device_id,
        compliant: true,
        last_seen: None,
        policy_applied: false,
    })
}
