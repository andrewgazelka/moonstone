//! MDM HTTP handlers.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use color_eyre::eyre::WrapErr as _;

use mdm_core::{CheckinMessage, Request, parse_checkin, parse_command_results};
use mdm_service::{Checkin, CommandAndReportResults};

/// Serialize a value to XML plist bytes.
fn to_plist_xml<T: serde::Serialize>(value: &T) -> color_eyre::eyre::Result<Vec<u8>> {
    let mut buf = Vec::new();
    plist::to_writer_xml(&mut buf, value).wrap_err("failed to serialize plist")?;
    Ok(buf)
}

/// Content type for MDM check-in messages.
#[allow(dead_code)]
const CHECKIN_CONTENT_TYPE: &str = "application/x-apple-aspen-mdm-checkin";

/// Handle MDM check-in requests.
pub async fn checkin_handler<S>(
    State(service): State<S>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse
where
    S: Checkin,
{
    match handle_checkin_inner(&service, &headers, &body).await {
        Ok(response) => (StatusCode::OK, response),
        Err(e) => {
            tracing::error!(error = %e, "check-in handler error");
            (StatusCode::INTERNAL_SERVER_ERROR, Vec::new())
        }
    }
}

async fn handle_checkin_inner<S: Checkin>(
    service: &S,
    headers: &HeaderMap,
    body: &[u8],
) -> color_eyre::eyre::Result<Vec<u8>> {
    // Extract certificate from headers
    let cert = extract_certificate(headers)?;

    // Parse check-in message
    let msg = parse_checkin(body)?;

    // Build request context
    let mut req = Request::new();
    if let Some(cert) = cert {
        req = req.with_certificate(cert);
    }

    // Resolve enrollment ID from message
    let enroll_id = match &msg {
        CheckinMessage::Authenticate(m) => m.enrollment.resolve(),
        CheckinMessage::TokenUpdate(m) => m.enrollment.resolve(),
        CheckinMessage::CheckOut(m) => m.enrollment.resolve(),
        CheckinMessage::UserAuthenticate(m) => m.enrollment.resolve(),
        CheckinMessage::SetBootstrapToken(m) => m.enrollment.resolve(),
        CheckinMessage::GetBootstrapToken(m) => m.enrollment.resolve(),
        CheckinMessage::DeclarativeManagement(m) => m.enrollment.resolve(),
        CheckinMessage::GetToken(m) => m.enrollment.resolve(),
    };

    if let Some(id) = enroll_id {
        req = req.with_enroll_id(id);
    }

    // Dispatch to service
    let response = match msg {
        CheckinMessage::Authenticate(ref m) => {
            service.authenticate(&req, m).await?;
            None
        }
        CheckinMessage::TokenUpdate(ref m) => {
            service.token_update(&req, m).await?;
            None
        }
        CheckinMessage::CheckOut(ref m) => {
            service.checkout(&req, m).await?;
            None
        }
        CheckinMessage::UserAuthenticate(ref m) => service.user_authenticate(&req, m).await?,
        CheckinMessage::SetBootstrapToken(ref m) => {
            service.set_bootstrap_token(&req, m).await?;
            None
        }
        CheckinMessage::GetBootstrapToken(ref m) => {
            let resp = service.get_bootstrap_token(&req, m).await?;
            match resp {
                Some(r) => Some(to_plist_xml(&r)?),
                None => None,
            }
        }
        CheckinMessage::DeclarativeManagement(ref m) => {
            service.declarative_management(&req, m).await?
        }
        CheckinMessage::GetToken(ref m) => {
            let resp = service.get_token(&req, m).await?;
            match resp {
                Some(r) => Some(to_plist_xml(&r)?),
                None => None,
            }
        }
    };

    Ok(response.unwrap_or_default())
}

/// Handle MDM command/report results requests.
pub async fn command_handler<S>(
    State(service): State<S>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse
where
    S: CommandAndReportResults,
{
    match handle_command_inner(&service, &headers, &body).await {
        Ok(response) => (StatusCode::OK, response),
        Err(e) => {
            tracing::error!(error = %e, "command handler error");
            (StatusCode::INTERNAL_SERVER_ERROR, Vec::new())
        }
    }
}

async fn handle_command_inner<S: CommandAndReportResults>(
    service: &S,
    headers: &HeaderMap,
    body: &[u8],
) -> color_eyre::eyre::Result<Vec<u8>> {
    // Extract certificate
    let cert = extract_certificate(headers)?;

    // Parse command results
    let results = parse_command_results(body)?;

    // Build request
    let mut req = Request::new();
    if let Some(cert) = cert {
        req = req.with_certificate(cert);
    }
    if let Some(id) = results.enrollment.resolve() {
        req = req.with_enroll_id(id);
    }

    // Get next command
    let next_cmd = service.command_and_report_results(&req, &results).await?;

    // Serialize response
    if let Some(cmd) = next_cmd {
        let response = to_plist_xml(&cmd)?;
        Ok(response)
    } else {
        Ok(Vec::new())
    }
}

/// Extract certificate from request headers.
fn extract_certificate(headers: &HeaderMap) -> color_eyre::eyre::Result<Option<Vec<u8>>> {
    // Try Mdm-Signature header first
    if let Some(sig) = headers.get("Mdm-Signature") {
        let _sig_str = sig.to_str().wrap_err("invalid Mdm-Signature header")?;
        // Note: Would need body for full verification
        // For now just return None
        return Ok(None);
    }

    // Try certificate header (RFC 9440 or PEM)
    for header_name in ["X-Ssl-Client-Cert", "X-Client-Cert", "Ssl-Client-Cert"] {
        if let Some(value) = headers.get(header_name) {
            let value_str = value.to_str().wrap_err("invalid cert header")?;

            // RFC 9440 format: :base64:
            if value_str.starts_with(':') {
                let cert = mdm_crypto::extract_rfc9440_cert(value_str)?;
                return Ok(Some(cert));
            }

            // URL-encoded PEM
            let cert = mdm_crypto::extract_pem_header(value_str)?;
            return Ok(Some(cert));
        }
    }

    Ok(None)
}
