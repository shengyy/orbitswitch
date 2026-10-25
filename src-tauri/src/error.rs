//! Error type shared by every backend module.
//!
//! Each variant carries a stable `code` that the frontend maps to a localized
//! message. The `message` is English diagnostic detail for logs and bug
//! reports; it must never contain tokens or other secrets.

use serde::{Serialize, Serializer, ser::SerializeStruct};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the official client could not be located")]
    ClientNotFound,
    #[error("sign in to a subscription account in the official client first")]
    SubscriptionLoginRequired,
    #[error("sign in to this account in Claude Desktop, then import it to bind both clients")]
    DesktopSignInRequired,
    #[error("the active client's token expired; open the client to renew its sign-in")]
    LiveTokenExpired,
    #[error("credential rollback failed; reopen the client and sign in again")]
    RollbackFailed,
    #[error("Antigravity is not installed or could not be located")]
    AntigravityNotFound,
    #[error("could not read the OAuth client from the installed Antigravity")]
    ClientDiscovery,
    #[error("authorization was cancelled")]
    OAuthCancelled,
    #[error("authorization timed out")]
    OAuthTimeout,
    #[error("authorization was denied: {0}")]
    OAuthDenied(String),
    #[error("The service rejected the stored authorization; sign in again")]
    TokenRevoked,
    #[error("network error: {0}")]
    Network(String),
    #[error("unexpected response from {endpoint}: HTTP {status}")]
    Http { endpoint: &'static str, status: u16 },
    #[error("credential store error: {0}")]
    SecretStore(String),
    #[error("{0} client process(es) did not exit")]
    ProcessStuck(usize),
    #[error("failed to launch the desktop client: {0}")]
    Launch(String),
    #[error("account switched, but the desktop client could not be opened: {0}")]
    LaunchAfterSwitch(String),
    #[error("account not found")]
    AccountNotFound,
    #[error("another operation is in progress")]
    Busy,
    #[error("app update failed: {0}")]
    Update(#[from] tauri_plugin_updater::Error),
    #[error("this update is no longer available; check again")]
    UpdateUnavailable,
    #[error("app updates require an installed bundle; install OrbitSwitch first")]
    UpdateNotInstalled,
    #[error("the written credential did not read back as the target account")]
    VerifyFailed,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid data: {0}")]
    Invalid(String),
}

impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Self::ClientNotFound => "client_not_found",
            Self::SubscriptionLoginRequired => "subscription_login_required",
            Self::DesktopSignInRequired => "desktop_sign_in_required",
            Self::LiveTokenExpired => "live_token_expired",
            Self::RollbackFailed => "rollback_failed",
            Self::AntigravityNotFound => "antigravity_not_found",
            Self::ClientDiscovery => "client_discovery",
            Self::OAuthCancelled => "oauth_cancelled",
            Self::OAuthTimeout => "oauth_timeout",
            Self::OAuthDenied(_) => "oauth_denied",
            Self::TokenRevoked => "token_revoked",
            Self::Network(_) => "network",
            Self::Http { .. } => "http",
            Self::SecretStore(_) => "secret_store",
            Self::ProcessStuck(_) => "process_stuck",
            Self::Launch(_) => "launch",
            Self::LaunchAfterSwitch(_) => "launch_after_switch",
            Self::AccountNotFound => "account_not_found",
            Self::Busy => "busy",
            Self::Update(_) => "update",
            Self::UpdateUnavailable => "update_unavailable",
            Self::UpdateNotInstalled => "update_not_installed",
            Self::VerifyFailed => "verify_failed",
            Self::Io(_) => "io",
            Self::Invalid(_) => "invalid",
        }
    }

    pub fn after_switch(self) -> Self {
        match self {
            Self::Launch(message) => Self::LaunchAfterSwitch(message),
            other => other,
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        // reqwest errors include the URL but never request bodies or headers.
        Self::Network(err.without_url().to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::Invalid(err.to_string())
    }
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("Error", 2)?;
        s.serialize_field("code", self.code())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

pub type Result<T> = std::result::Result<T, Error>;
