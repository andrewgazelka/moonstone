//! Certificate authentication middleware.

use color_eyre::eyre::WrapErr as _;
use mdm_core::{
    Authenticate, BootstrapTokenResponse, CheckOut, Command, CommandResults, DeclarativeManagement,
    GetBootstrapToken, GetToken, GetTokenResponse, Request, SetBootstrapToken, TokenUpdate,
    UserAuthenticate,
};
use mdm_storage::CertAuthStore;

use crate::{Checkin, CommandAndReportResults};

/// Certificate authentication service wrapper.
///
/// Validates that requests come from enrolled certificates.
pub struct CertAuthService<S, I> {
    store: S,
    inner: I,
}

impl<S, I> CertAuthService<S, I> {
    /// Create a new cert auth service.
    pub fn new(store: S, inner: I) -> Self {
        Self { store, inner }
    }
}

impl<S, I> CertAuthService<S, I>
where
    S: CertAuthStore,
{
    fn validate_cert(&self, req: &Request) -> color_eyre::eyre::Result<()> {
        let id = req.require_enroll_id()?;

        let cert = req
            .certificate
            .as_ref()
            .ok_or_else(|| color_eyre::eyre::eyre!("no certificate in request"))?;

        let cert_hash = mdm_crypto::cert_hash(cert);

        if !self.store.has_cert_auth(id, &cert_hash)? {
            color_eyre::eyre::bail!("certificate not authorized for enrollment {}", id.id);
        }

        Ok(())
    }
}

impl<S: CertAuthStore, I: Checkin> Checkin for CertAuthService<S, I> {
    async fn authenticate(
        &self,
        req: &Request,
        msg: &Authenticate,
    ) -> color_eyre::eyre::Result<()> {
        // On Authenticate, associate the certificate with the enrollment
        if let (Some(id), Some(cert)) = (req.enroll_id.as_ref(), req.certificate.as_ref()) {
            let cert_hash = mdm_crypto::cert_hash(cert);
            self.store
                .associate_cert(id, &cert_hash)
                .wrap_err("failed to associate certificate")?;
        }

        self.inner.authenticate(req, msg).await
    }

    async fn token_update(&self, req: &Request, msg: &TokenUpdate) -> color_eyre::eyre::Result<()> {
        self.validate_cert(req)?;
        self.inner.token_update(req, msg).await
    }

    async fn checkout(&self, req: &Request, msg: &CheckOut) -> color_eyre::eyre::Result<()> {
        self.validate_cert(req)?;
        self.inner.checkout(req, msg).await
    }

    async fn user_authenticate(
        &self,
        req: &Request,
        msg: &UserAuthenticate,
    ) -> color_eyre::eyre::Result<Option<Vec<u8>>> {
        self.validate_cert(req)?;
        self.inner.user_authenticate(req, msg).await
    }

    async fn set_bootstrap_token(
        &self,
        req: &Request,
        msg: &SetBootstrapToken,
    ) -> color_eyre::eyre::Result<()> {
        self.validate_cert(req)?;
        self.inner.set_bootstrap_token(req, msg).await
    }

    async fn get_bootstrap_token(
        &self,
        req: &Request,
        msg: &GetBootstrapToken,
    ) -> color_eyre::eyre::Result<Option<BootstrapTokenResponse>> {
        self.validate_cert(req)?;
        self.inner.get_bootstrap_token(req, msg).await
    }

    async fn declarative_management(
        &self,
        req: &Request,
        msg: &DeclarativeManagement,
    ) -> color_eyre::eyre::Result<Option<Vec<u8>>> {
        self.validate_cert(req)?;
        self.inner.declarative_management(req, msg).await
    }

    async fn get_token(
        &self,
        req: &Request,
        msg: &GetToken,
    ) -> color_eyre::eyre::Result<Option<GetTokenResponse>> {
        self.validate_cert(req)?;
        self.inner.get_token(req, msg).await
    }
}

impl<S: CertAuthStore, I: CommandAndReportResults> CommandAndReportResults
    for CertAuthService<S, I>
{
    async fn command_and_report_results(
        &self,
        req: &Request,
        results: &CommandResults,
    ) -> color_eyre::eyre::Result<Option<Command>> {
        self.validate_cert(req)?;
        self.inner.command_and_report_results(req, results).await
    }
}
