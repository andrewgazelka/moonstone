//! MDM command types.

use crate::Enrollment;

/// MDM command to send to device.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Command {
    /// Unique command identifier.
    pub command_uuid: String,

    /// Command payload.
    pub command: CommandPayload,
}

/// Command payload wrapper.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CommandPayload {
    /// Request type (e.g., "DeviceInformation", "InstallProfile").
    pub request_type: String,

    /// Additional command-specific fields stored as raw plist.
    #[serde(flatten)]
    pub data: std::collections::HashMap<String, plist::Value>,
}

/// Command results reported by device.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CommandResults {
    /// Enrollment identification.
    #[serde(flatten)]
    pub enrollment: Enrollment,

    /// Command UUID being reported.
    pub command_uuid: String,

    /// Status of command execution.
    pub status: CommandStatus,

    /// Error chain (if failed).
    #[serde(default)]
    pub error_chain: Vec<ErrorChainItem>,

    /// Raw message for storage.
    #[serde(skip)]
    pub raw: Vec<u8>,
}

/// Command execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CommandStatus {
    /// Command executed successfully.
    Acknowledged,
    /// Command failed.
    Error,
    /// Command format error.
    CommandFormatError,
    /// Device is busy, try later.
    NotNow,
    /// Idle (no command was pending).
    Idle,
}

impl std::fmt::Display for CommandStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Acknowledged => write!(f, "Acknowledged"),
            Self::Error => write!(f, "Error"),
            Self::CommandFormatError => write!(f, "CommandFormatError"),
            Self::NotNow => write!(f, "NotNow"),
            Self::Idle => write!(f, "Idle"),
        }
    }
}

/// Error chain item from device.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ErrorChainItem {
    /// Error code.
    pub error_code: i32,

    /// Error domain.
    pub error_domain: String,

    /// Localized description.
    #[serde(default)]
    pub localized_description: Option<String>,

    /// US English description.
    #[serde(default, rename = "USEnglishDescription")]
    pub us_english_description: Option<String>,
}

/// Queued command in storage.
#[derive(Debug, Clone)]
pub struct QueuedCommand {
    /// Command UUID.
    pub uuid: String,
    /// Raw command plist.
    pub command: Vec<u8>,
    /// When command was queued.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Parse command results from plist bytes.
pub fn parse_command_results(data: &[u8]) -> color_eyre::eyre::Result<CommandResults> {
    use color_eyre::eyre::WrapErr as _;

    plist::from_bytes(data).wrap_err("failed to parse command results")
}

/// Create a new command with generated UUID.
pub fn new_command(request_type: &str) -> Command {
    Command {
        command_uuid: uuid::Uuid::new_v4().to_string(),
        command: CommandPayload {
            request_type: request_type.to_string(),
            data: std::collections::HashMap::new(),
        },
    }
}

/// Serialize command to plist bytes.
pub fn serialize_command(cmd: &Command) -> color_eyre::eyre::Result<Vec<u8>> {
    use color_eyre::eyre::WrapErr as _;

    let mut buf = Vec::new();
    plist::to_writer_xml(&mut buf, cmd).wrap_err("failed to serialize command")?;
    Ok(buf)
}

/// Convert a serializable value to a plist Value.
pub fn to_plist_value<T: serde::Serialize>(value: &T) -> color_eyre::eyre::Result<plist::Value> {
    use color_eyre::eyre::WrapErr as _;

    plist::to_value(value).wrap_err("failed to convert to plist value")
}
