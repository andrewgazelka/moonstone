//! APNs push implementation using the a2 crate.

use a2::NotificationBuilder as _;
use color_eyre::eyre::WrapErr as _;
use mdm_core::{PushInfo, PushResult};

use crate::Pusher;

/// APNs pusher using certificate authentication.
pub struct ApnsPusher {
    client: a2::Client,
    #[allow(dead_code)]
    endpoint: a2::Endpoint,
}

impl ApnsPusher {
    /// Create a new APNs pusher from PKCS12 certificate bytes and password.
    pub fn new(pkcs12_der: &[u8], password: &str) -> color_eyre::eyre::Result<Self> {
        let mut cursor = std::io::Cursor::new(pkcs12_der);
        let config = a2::ClientConfig::default();

        let client = a2::Client::certificate(&mut cursor, password, config)
            .wrap_err("failed to create APNs client")?;

        Ok(Self {
            client,
            endpoint: a2::Endpoint::Production,
        })
    }

    /// Create a new APNs pusher for sandbox environment.
    pub fn sandbox(pkcs12_der: &[u8], password: &str) -> color_eyre::eyre::Result<Self> {
        let mut cursor = std::io::Cursor::new(pkcs12_der);
        let config = a2::ClientConfig::default();

        let client = a2::Client::certificate(&mut cursor, password, config)
            .wrap_err("failed to create APNs client")?;

        Ok(Self {
            client,
            endpoint: a2::Endpoint::Sandbox,
        })
    }
}

impl Pusher for ApnsPusher {
    async fn push(&self, infos: &[&PushInfo]) -> Vec<PushResult> {
        let mut results = Vec::with_capacity(infos.len());

        for info in infos {
            let result = self.push_single(info).await;
            results.push(result);
        }

        results
    }
}

impl ApnsPusher {
    async fn push_single(&self, info: &PushInfo) -> PushResult {
        // MDM push payload is just {"mdm": "<push_magic>"}
        let body = format!(r#"{{"mdm":"{}"}}"#, info.push_magic);
        let token = info.token_hex();

        let payload = a2::DefaultNotificationBuilder::new().set_body(&body).build(
            &token,
            a2::NotificationOptions {
                apns_topic: Some(&info.topic),
                ..Default::default()
            },
        );

        match self.client.send(payload).await {
            Ok(response) => {
                let apns_id = response.apns_id.unwrap_or_default();
                PushResult::success(token, apns_id)
            }
            Err(e) => PushResult::failure(token, e),
        }
    }
}

/// Push service that resolves enrollment IDs to push info.
pub struct PushService<S, P> {
    store: S,
    pusher: P,
}

impl<S, P> PushService<S, P>
where
    S: mdm_storage::PushStore,
    P: Pusher,
{
    /// Create a new push service.
    pub fn new(store: S, pusher: P) -> Self {
        Self { store, pusher }
    }

    /// Push to enrollments by ID.
    pub async fn push_by_ids(
        &self,
        ids: &[&mdm_core::EnrollId],
    ) -> color_eyre::eyre::Result<Vec<PushResult>> {
        let infos = self.store.get_push_infos(ids)?;
        let info_refs: Vec<&PushInfo> = infos.iter().collect();

        Ok(self.pusher.push(&info_refs).await)
    }
}

// Re-export for convenience
pub use mdm_storage;
