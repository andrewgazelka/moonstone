//! Multi-service composition.

use mdm_core::{
    Authenticate, BootstrapTokenResponse, CheckOut, Command, CommandResults, DeclarativeManagement,
    GetBootstrapToken, GetToken, GetTokenResponse, Request, SetBootstrapToken, TokenUpdate,
    UserAuthenticate,
};

use crate::{Checkin, CommandAndReportResults};

/// Compose multiple services - primary returns values, others run as side-effects.
pub struct MultiService<P, S> {
    primary: P,
    secondary: Vec<S>,
}

impl<P, S> MultiService<P, S> {
    /// Create a new multi-service with a primary and secondary services.
    pub fn new(primary: P, secondary: Vec<S>) -> Self {
        Self { primary, secondary }
    }

    /// Add a secondary service.
    pub fn with_secondary(mut self, service: S) -> Self {
        self.secondary.push(service);
        self
    }
}

impl<P: Checkin, S: Checkin> Checkin for MultiService<P, S> {
    async fn authenticate(
        &self,
        req: &Request,
        msg: &Authenticate,
    ) -> color_eyre::eyre::Result<()> {
        // Primary first
        self.primary.authenticate(req, msg).await?;

        // Fire-and-forget for secondaries
        for secondary in &self.secondary {
            if let Err(e) = secondary.authenticate(req, msg).await {
                tracing::warn!(error = %e, "secondary service authenticate failed");
            }
        }

        Ok(())
    }

    async fn token_update(&self, req: &Request, msg: &TokenUpdate) -> color_eyre::eyre::Result<()> {
        self.primary.token_update(req, msg).await?;

        for secondary in &self.secondary {
            if let Err(e) = secondary.token_update(req, msg).await {
                tracing::warn!(error = %e, "secondary service token_update failed");
            }
        }

        Ok(())
    }

    async fn checkout(&self, req: &Request, msg: &CheckOut) -> color_eyre::eyre::Result<()> {
        self.primary.checkout(req, msg).await?;

        for secondary in &self.secondary {
            if let Err(e) = secondary.checkout(req, msg).await {
                tracing::warn!(error = %e, "secondary service checkout failed");
            }
        }

        Ok(())
    }

    async fn user_authenticate(
        &self,
        req: &Request,
        msg: &UserAuthenticate,
    ) -> color_eyre::eyre::Result<Option<Vec<u8>>> {
        let result = self.primary.user_authenticate(req, msg).await?;

        for secondary in &self.secondary {
            let _ = secondary.user_authenticate(req, msg).await;
        }

        Ok(result)
    }

    async fn set_bootstrap_token(
        &self,
        req: &Request,
        msg: &SetBootstrapToken,
    ) -> color_eyre::eyre::Result<()> {
        self.primary.set_bootstrap_token(req, msg).await?;

        for secondary in &self.secondary {
            let _ = secondary.set_bootstrap_token(req, msg).await;
        }

        Ok(())
    }

    async fn get_bootstrap_token(
        &self,
        req: &Request,
        msg: &GetBootstrapToken,
    ) -> color_eyre::eyre::Result<Option<BootstrapTokenResponse>> {
        self.primary.get_bootstrap_token(req, msg).await
    }

    async fn declarative_management(
        &self,
        req: &Request,
        msg: &DeclarativeManagement,
    ) -> color_eyre::eyre::Result<Option<Vec<u8>>> {
        self.primary.declarative_management(req, msg).await
    }

    async fn get_token(
        &self,
        req: &Request,
        msg: &GetToken,
    ) -> color_eyre::eyre::Result<Option<GetTokenResponse>> {
        self.primary.get_token(req, msg).await
    }
}

impl<P: CommandAndReportResults, S: CommandAndReportResults> CommandAndReportResults
    for MultiService<P, S>
{
    async fn command_and_report_results(
        &self,
        req: &Request,
        results: &CommandResults,
    ) -> color_eyre::eyre::Result<Option<Command>> {
        let cmd = self
            .primary
            .command_and_report_results(req, results)
            .await?;

        for secondary in &self.secondary {
            let _ = secondary.command_and_report_results(req, results).await;
        }

        Ok(cmd)
    }
}
