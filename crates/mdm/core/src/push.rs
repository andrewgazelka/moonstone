//! Push notification types.

/// Push notification info for a device.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PushInfo {
    /// APNs push token (raw bytes).
    pub token: Vec<u8>,
    /// Push magic string.
    pub push_magic: String,
    /// APNs topic (from push certificate).
    pub topic: String,
}

impl PushInfo {
    /// Get push token as hex string.
    pub fn token_hex(&self) -> String {
        hex_encode(&self.token)
    }
}

/// Result of a push notification attempt.
#[derive(Debug, Clone)]
pub struct PushResult {
    /// Enrollment ID that was pushed.
    pub enrollment_id: String,
    /// APNs response ID (if successful).
    pub apns_id: Option<String>,
    /// Error (if failed).
    pub error: Option<String>,
}

impl PushResult {
    /// Create a successful push result.
    pub fn success(enrollment_id: String, apns_id: String) -> Self {
        Self {
            enrollment_id,
            apns_id: Some(apns_id),
            error: None,
        }
    }

    /// Create a failed push result.
    pub fn failure(enrollment_id: String, error: impl std::fmt::Display) -> Self {
        Self {
            enrollment_id,
            apns_id: None,
            error: Some(error.to_string()),
        }
    }

    /// Check if push was successful.
    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
