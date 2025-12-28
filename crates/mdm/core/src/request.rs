//! MDM request context.

use crate::EnrollId;

/// MDM request context.
///
/// Carries enrollment identification and certificate info through the service layer.
#[derive(Debug, Clone)]
pub struct Request {
    /// Resolved enrollment ID.
    pub enroll_id: Option<EnrollId>,

    /// Device certificate (DER-encoded).
    pub certificate: Option<Vec<u8>>,

    /// URL query parameters.
    pub params: std::collections::HashMap<String, String>,
}

impl Request {
    /// Create a new empty request.
    pub fn new() -> Self {
        Self {
            enroll_id: None,
            certificate: None,
            params: std::collections::HashMap::new(),
        }
    }

    /// Set the enrollment ID.
    pub fn with_enroll_id(mut self, id: EnrollId) -> Self {
        self.enroll_id = Some(id);
        self
    }

    /// Set the certificate.
    pub fn with_certificate(mut self, cert: Vec<u8>) -> Self {
        self.certificate = Some(cert);
        self
    }

    /// Add a query parameter.
    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }

    /// Get the enrollment ID, returning an error if not set.
    pub fn require_enroll_id(&self) -> color_eyre::eyre::Result<&EnrollId> {
        self.enroll_id
            .as_ref()
            .ok_or_else(|| color_eyre::eyre::eyre!("enrollment ID not resolved"))
    }
}

impl Default for Request {
    fn default() -> Self {
        Self::new()
    }
}
