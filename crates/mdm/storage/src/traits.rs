//! Storage traits.

use mdm_core::{
    BootstrapTokenResponse, CheckinMessage, CommandResults, EnrollId, PushInfo, QueuedCommand,
};

/// Check-in storage operations.
pub trait CheckinStore: Send + Sync {
    /// Store an Authenticate message and clear command queue.
    fn store_authenticate(
        &self,
        id: &EnrollId,
        msg: &mdm_core::Authenticate,
    ) -> color_eyre::eyre::Result<()>;

    /// Store a TokenUpdate message and enable enrollment.
    fn store_token_update(
        &self,
        id: &EnrollId,
        msg: &mdm_core::TokenUpdate,
    ) -> color_eyre::eyre::Result<()>;

    /// Store a CheckOut message and disable enrollment.
    fn store_checkout(
        &self,
        id: &EnrollId,
        msg: &mdm_core::CheckOut,
    ) -> color_eyre::eyre::Result<()>;

    /// Check if an enrollment is disabled.
    fn is_disabled(&self, id: &EnrollId) -> color_eyre::eyre::Result<bool>;

    /// Disable an enrollment.
    fn disable(&self, id: &EnrollId) -> color_eyre::eyre::Result<()>;
}

/// Command storage operations.
pub trait CommandStore: Send + Sync {
    /// Enqueue a command for an enrollment.
    fn enqueue_command(&self, id: &EnrollId, command: &[u8]) -> color_eyre::eyre::Result<String>;

    /// Get the next pending command for an enrollment.
    fn next_command(&self, id: &EnrollId) -> color_eyre::eyre::Result<Option<QueuedCommand>>;

    /// Store command results and remove from queue.
    fn store_result(&self, id: &EnrollId, results: &CommandResults)
    -> color_eyre::eyre::Result<()>;

    /// Clear all pending commands for an enrollment.
    fn clear_queue(&self, id: &EnrollId) -> color_eyre::eyre::Result<()>;
}

/// Bootstrap token storage.
pub trait BootstrapTokenStore: Send + Sync {
    /// Store a bootstrap token.
    fn store_bootstrap_token(&self, id: &EnrollId, token: &[u8]) -> color_eyre::eyre::Result<()>;

    /// Get a bootstrap token.
    fn get_bootstrap_token(&self, id: &EnrollId) -> color_eyre::eyre::Result<Option<Vec<u8>>>;

    /// Delete a bootstrap token.
    fn delete_bootstrap_token(&self, id: &EnrollId) -> color_eyre::eyre::Result<()>;
}

/// Push info storage.
pub trait PushStore: Send + Sync {
    /// Get push info for an enrollment.
    fn get_push_info(&self, id: &EnrollId) -> color_eyre::eyre::Result<Option<PushInfo>>;

    /// Get push info for multiple enrollments.
    fn get_push_infos(&self, ids: &[&EnrollId]) -> color_eyre::eyre::Result<Vec<PushInfo>>;
}

/// Push certificate storage.
pub trait PushCertStore: Send + Sync {
    /// Store a push certificate.
    fn store_push_cert(
        &self,
        topic: &str,
        cert_pem: &str,
        key_pem: &str,
    ) -> color_eyre::eyre::Result<()>;

    /// Get a push certificate by topic.
    fn get_push_cert(&self, topic: &str) -> color_eyre::eyre::Result<Option<(String, String)>>;
}

/// Certificate authentication storage.
pub trait CertAuthStore: Send + Sync {
    /// Associate a certificate hash with an enrollment.
    fn associate_cert(&self, id: &EnrollId, cert_hash: &[u8]) -> color_eyre::eyre::Result<()>;

    /// Check if a certificate is associated with an enrollment.
    fn has_cert_auth(&self, id: &EnrollId, cert_hash: &[u8]) -> color_eyre::eyre::Result<bool>;
}

/// Combined storage trait.
pub trait AllStorage:
    CheckinStore + CommandStore + BootstrapTokenStore + PushStore + PushCertStore + CertAuthStore
{
}

impl<T> AllStorage for T where
    T: CheckinStore
        + CommandStore
        + BootstrapTokenStore
        + PushStore
        + PushCertStore
        + CertAuthStore
{
}
