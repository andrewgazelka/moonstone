//! Core NanoMDM service implementation.

use color_eyre::eyre::WrapErr as _;
use mdm_core::{
    Authenticate, BootstrapTokenResponse, CheckOut, Command, CommandResults, DeclarativeManagement,
    GetBootstrapToken, GetToken, GetTokenResponse, Request, SetBootstrapToken, TokenUpdate,
    UserAuthenticate,
};
use mdm_storage::AllStorage;

use crate::{Checkin, CommandAndReportResults};

/// Core MDM service implementation.
#[derive(Clone)]
pub struct NanoMdm<S> {
    store: S,
}

impl<S> NanoMdm<S> {
    /// Create a new NanoMdm service.
    pub fn new(store: S) -> Self {
        Self { store }
    }
}

impl<S: AllStorage> Checkin for NanoMdm<S> {
    async fn authenticate(
        &self,
        req: &Request,
        msg: &Authenticate,
    ) -> color_eyre::eyre::Result<()> {
        let id = req.require_enroll_id()?;

        tracing::info!(enrollment_id = %id.id, "processing authenticate");

        // Clear bootstrap token on re-enrollment
        self.store
            .delete_bootstrap_token(id)
            .wrap_err("failed to delete bootstrap token")?;

        // Store authenticate and disable until TokenUpdate
        self.store
            .store_authenticate(id, msg)
            .wrap_err("failed to store authenticate")?;

        Ok(())
    }

    async fn token_update(&self, req: &Request, msg: &TokenUpdate) -> color_eyre::eyre::Result<()> {
        let id = req.require_enroll_id()?;

        tracing::info!(enrollment_id = %id.id, "processing token update");

        self.store
            .store_token_update(id, msg)
            .wrap_err("failed to store token update")?;

        Ok(())
    }

    async fn checkout(&self, req: &Request, msg: &CheckOut) -> color_eyre::eyre::Result<()> {
        let id = req.require_enroll_id()?;

        tracing::info!(enrollment_id = %id.id, "processing checkout");

        self.store
            .store_checkout(id, msg)
            .wrap_err("failed to store checkout")?;

        Ok(())
    }

    async fn user_authenticate(
        &self,
        req: &Request,
        _msg: &UserAuthenticate,
    ) -> color_eyre::eyre::Result<Option<Vec<u8>>> {
        let id = req.require_enroll_id()?;

        tracing::info!(enrollment_id = %id.id, "processing user authenticate");

        // Default: no digest challenge
        Ok(None)
    }

    async fn set_bootstrap_token(
        &self,
        req: &Request,
        msg: &SetBootstrapToken,
    ) -> color_eyre::eyre::Result<()> {
        let id = req.require_enroll_id()?;

        tracing::info!(enrollment_id = %id.id, "storing bootstrap token");

        self.store
            .store_bootstrap_token(id, &msg.bootstrap_token)
            .wrap_err("failed to store bootstrap token")?;

        Ok(())
    }

    async fn get_bootstrap_token(
        &self,
        req: &Request,
        _msg: &GetBootstrapToken,
    ) -> color_eyre::eyre::Result<Option<BootstrapTokenResponse>> {
        let id = req.require_enroll_id()?;

        tracing::info!(enrollment_id = %id.id, "retrieving bootstrap token");

        let token = self
            .store
            .get_bootstrap_token(id)
            .wrap_err("failed to get bootstrap token")?;

        Ok(token.map(|t| BootstrapTokenResponse { bootstrap_token: t }))
    }

    async fn declarative_management(
        &self,
        req: &Request,
        msg: &DeclarativeManagement,
    ) -> color_eyre::eyre::Result<Option<Vec<u8>>> {
        let id = req.require_enroll_id()?;

        tracing::info!(
            enrollment_id = %id.id,
            endpoint = ?msg.endpoint,
            "processing declarative management"
        );

        // DDM handling would go here
        // For now, return empty response
        Ok(None)
    }

    async fn get_token(
        &self,
        req: &Request,
        msg: &GetToken,
    ) -> color_eyre::eyre::Result<Option<GetTokenResponse>> {
        let id = req.require_enroll_id()?;

        tracing::info!(
            enrollment_id = %id.id,
            service_type = %msg.token_service_type,
            "processing get token"
        );

        // Token exchange would go here
        Ok(None)
    }
}

impl<S: AllStorage> CommandAndReportResults for NanoMdm<S> {
    async fn command_and_report_results(
        &self,
        req: &Request,
        results: &CommandResults,
    ) -> color_eyre::eyre::Result<Option<Command>> {
        let id = req.require_enroll_id()?;

        // Store results if this is a response to a command
        if !results.command_uuid.is_empty() {
            tracing::info!(
                enrollment_id = %id.id,
                command_uuid = %results.command_uuid,
                status = %results.status,
                "storing command results"
            );

            self.store
                .store_result(id, results)
                .wrap_err("failed to store command results")?;
        }

        // Get next pending command
        let next = self
            .store
            .next_command(id)
            .wrap_err("failed to get next command")?;

        if let Some(queued) = next {
            tracing::info!(
                enrollment_id = %id.id,
                command_uuid = %queued.uuid,
                "sending next command"
            );

            // Parse the stored command
            let cmd: Command =
                plist::from_bytes(&queued.command).wrap_err("failed to parse stored command")?;

            return Ok(Some(cmd));
        }

        Ok(None)
    }
}
