#![deny(unsafe_code)]

pub mod account;
pub mod api_contract;
pub mod api_key_service;
pub mod config;
pub mod csrf;
pub mod error;
pub mod event_bus;
pub mod magic_link;
pub mod mail;
pub mod models;
pub mod oidc;
pub mod openapi;
pub mod otp;
pub mod service;
pub mod totp_service;
pub mod traits;
pub mod ui;
pub mod webhook;

pub use account::AccountService;
pub use api_contract::{AuthApiCompatibilityNotes, LoginRequest, LoginResponse, SignupRequest};
pub use api_key_service::ApiKeyManager;
pub use config::{AuthConfig, AuthConfigBuilder};
pub use error::{AuthError, AuthResult};
pub use magic_link::MagicLinkService;
pub use models::{
    AccessToken, ApiKey, CsrfToken, EmailVerificationToken, Event, EventType, IssuedApiKey,
    MagicLinkToken, OtpRecord, PasswordResetToken, RefreshToken, Session, TenantId, TotpSetup,
    User, UserId,
};
pub use otp::OtpService;
pub use service::{AuthService, RegisterOutcome};

pub mod adapters {
    #[cfg(feature = "axum")]
    pub mod axum;

    #[cfg(feature = "actix")]
    pub mod actix;

    #[cfg(feature = "warp")]
    pub mod warp;
}

#[cfg(test)]
mod tests;
