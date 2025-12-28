//! MDM check-in message types.

use crate::Enrollment;

/// Check-in message types from devices.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "MessageType")]
pub enum CheckinMessage {
    /// Initial device authentication.
    Authenticate(Authenticate),
    /// Push token update (enrollment complete).
    TokenUpdate(TokenUpdate),
    /// Device unenrollment.
    CheckOut(CheckOut),
    /// User authentication challenge.
    UserAuthenticate(UserAuthenticate),
    /// Set bootstrap token.
    SetBootstrapToken(SetBootstrapToken),
    /// Get bootstrap token.
    GetBootstrapToken(GetBootstrapToken),
    /// Declarative Management.
    DeclarativeManagement(DeclarativeManagement),
    /// Get token for services.
    GetToken(GetToken),
}

/// Authenticate message - initial device identity.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Authenticate {
    /// Enrollment identification.
    #[serde(flatten)]
    pub enrollment: Enrollment,

    /// Device topic for push notifications.
    pub topic: String,

    /// Build version.
    #[serde(default)]
    pub build_version: Option<String>,

    /// OS version.
    #[serde(default, rename = "OSVersion")]
    pub os_version: Option<String>,

    /// Product name.
    #[serde(default)]
    pub product_name: Option<String>,

    /// Serial number.
    #[serde(default)]
    pub serial_number: Option<String>,

    /// Device name.
    #[serde(default)]
    pub device_name: Option<String>,

    /// Model.
    #[serde(default)]
    pub model: Option<String>,

    /// Model name.
    #[serde(default)]
    pub model_name: Option<String>,

    /// Raw message for storage.
    #[serde(skip)]
    pub raw: Vec<u8>,
}

/// TokenUpdate message - push token registration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TokenUpdate {
    /// Enrollment identification.
    #[serde(flatten)]
    pub enrollment: Enrollment,

    /// Device topic for push notifications.
    pub topic: String,

    /// Push token (hex-encoded).
    #[serde(with = "hex_bytes")]
    pub token: Vec<u8>,

    /// Push magic string.
    pub push_magic: String,

    /// Unlock token (optional).
    #[serde(default)]
    pub unlock_token: Option<Vec<u8>>,

    /// Awaiting configuration (DEP).
    #[serde(default)]
    pub awaiting_configuration: bool,

    /// Raw message for storage.
    #[serde(skip)]
    pub raw: Vec<u8>,
}

/// CheckOut message - device unenrollment.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CheckOut {
    /// Enrollment identification.
    #[serde(flatten)]
    pub enrollment: Enrollment,

    /// Device topic.
    pub topic: String,

    /// Raw message for storage.
    #[serde(skip)]
    pub raw: Vec<u8>,
}

/// UserAuthenticate message - user identity challenge.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UserAuthenticate {
    /// Enrollment identification.
    #[serde(flatten)]
    pub enrollment: Enrollment,

    /// Digest response (optional).
    #[serde(default)]
    pub digest_response: Option<String>,

    /// Raw message for storage.
    #[serde(skip)]
    pub raw: Vec<u8>,
}

/// SetBootstrapToken message.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SetBootstrapToken {
    /// Enrollment identification.
    #[serde(flatten)]
    pub enrollment: Enrollment,

    /// Bootstrap token (base64-encoded in plist).
    pub bootstrap_token: Vec<u8>,

    /// Raw message for storage.
    #[serde(skip)]
    pub raw: Vec<u8>,
}

/// GetBootstrapToken message.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetBootstrapToken {
    /// Enrollment identification.
    #[serde(flatten)]
    pub enrollment: Enrollment,

    /// Raw message for storage.
    #[serde(skip)]
    pub raw: Vec<u8>,
}

/// Bootstrap token response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BootstrapTokenResponse {
    /// Bootstrap token.
    pub bootstrap_token: Vec<u8>,
}

/// DeclarativeManagement message.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DeclarativeManagement {
    /// Enrollment identification.
    #[serde(flatten)]
    pub enrollment: Enrollment,

    /// DDM endpoint being accessed.
    #[serde(default)]
    pub endpoint: Option<String>,

    /// DDM data payload.
    #[serde(default)]
    pub data: Option<Vec<u8>>,

    /// Raw message for storage.
    #[serde(skip)]
    pub raw: Vec<u8>,
}

/// GetToken message - token exchange for services.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetToken {
    /// Enrollment identification.
    #[serde(flatten)]
    pub enrollment: Enrollment,

    /// Token service type.
    pub token_service_type: String,

    /// Raw message for storage.
    #[serde(skip)]
    pub raw: Vec<u8>,
}

/// GetToken response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetTokenResponse {
    /// Token data.
    pub token_data: Vec<u8>,
}

/// Helper module for hex-encoded bytes in plist.
mod hex_bytes {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(bytes)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Plist Data type deserializes as bytes directly
        Vec::<u8>::deserialize(deserializer)
    }
}

/// Parse a check-in message from plist bytes.
pub fn parse_checkin(data: &[u8]) -> color_eyre::eyre::Result<CheckinMessage> {
    use color_eyre::eyre::WrapErr as _;

    plist::from_bytes(data).wrap_err("failed to parse check-in message")
}
