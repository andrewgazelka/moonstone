//! Push notification traits.

use mdm_core::{PushInfo, PushResult};

/// Low-level push notification sender.
#[trait_variant::make(Send)]
pub trait Pusher: Send + Sync {
    /// Push notifications to devices.
    async fn push(&self, infos: &[&PushInfo]) -> Vec<PushResult>;
}

/// High-level push provider that resolves enrollment IDs.
#[trait_variant::make(Send)]
pub trait PushProvider: Send + Sync {
    /// Push notifications by enrollment ID.
    async fn push_by_id(&self, ids: &[&str]) -> Vec<PushResult>;
}
