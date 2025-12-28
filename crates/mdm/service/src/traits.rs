//! Service traits.

use mdm_core::{
    Authenticate, BootstrapTokenResponse, CheckOut, Command, CommandResults, DeclarativeManagement,
    GetBootstrapToken, GetToken, GetTokenResponse, Request, SetBootstrapToken, TokenUpdate,
    UserAuthenticate,
};

/// Check-in service trait.
#[trait_variant::make(Send)]
pub trait Checkin: Send + Sync {
    /// Handle Authenticate message.
    async fn authenticate(&self, req: &Request, msg: &Authenticate)
    -> color_eyre::eyre::Result<()>;

    /// Handle TokenUpdate message.
    async fn token_update(&self, req: &Request, msg: &TokenUpdate) -> color_eyre::eyre::Result<()>;

    /// Handle CheckOut message.
    async fn checkout(&self, req: &Request, msg: &CheckOut) -> color_eyre::eyre::Result<()>;

    /// Handle UserAuthenticate message.
    async fn user_authenticate(
        &self,
        req: &Request,
        msg: &UserAuthenticate,
    ) -> color_eyre::eyre::Result<Option<Vec<u8>>>;

    /// Handle SetBootstrapToken message.
    async fn set_bootstrap_token(
        &self,
        req: &Request,
        msg: &SetBootstrapToken,
    ) -> color_eyre::eyre::Result<()>;

    /// Handle GetBootstrapToken message.
    async fn get_bootstrap_token(
        &self,
        req: &Request,
        msg: &GetBootstrapToken,
    ) -> color_eyre::eyre::Result<Option<BootstrapTokenResponse>>;

    /// Handle DeclarativeManagement message.
    async fn declarative_management(
        &self,
        req: &Request,
        msg: &DeclarativeManagement,
    ) -> color_eyre::eyre::Result<Option<Vec<u8>>>;

    /// Handle GetToken message.
    async fn get_token(
        &self,
        req: &Request,
        msg: &GetToken,
    ) -> color_eyre::eyre::Result<Option<GetTokenResponse>>;
}

/// Command and report results service trait.
#[trait_variant::make(Send)]
pub trait CommandAndReportResults: Send + Sync {
    /// Handle command results and return next command.
    async fn command_and_report_results(
        &self,
        req: &Request,
        results: &CommandResults,
    ) -> color_eyre::eyre::Result<Option<Command>>;
}

/// Combined check-in and command service.
pub trait CheckinAndCommand: Checkin + CommandAndReportResults {}

impl<T: Checkin + CommandAndReportResults> CheckinAndCommand for T {}
