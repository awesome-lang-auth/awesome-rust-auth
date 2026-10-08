// `#[async_trait]` (0.1.89, the version in Cargo.lock) puts a bare `#[must_use]`
// on every trait method it rewrites, and the rewritten return type,
// `Pin<Box<dyn Future>>`, is already `#[must_use]`. Since Rust 1.99, clippy
// reports that as `double_must_use`. The attribute comes from the macro, not
// from this file, so the lint is silenced for this module only.
#![allow(clippy::double_must_use)]

use async_trait::async_trait;

use crate::{error::AuthResult, models::*};

/// Persistence contract for user lifecycle operations.
#[async_trait]
pub trait UserStore: Send + Sync {
    /// Creates a user record.
    async fn create_user(&self, user: User) -> AuthResult<User>;
    /// Finds a user by tenant and email.
    async fn get_user_by_email(
        &self,
        tenant_id: &TenantId,
        email: &str,
    ) -> AuthResult<Option<User>>;
    /// Finds a user by id.
    async fn get_user_by_id(&self, user_id: &UserId) -> AuthResult<Option<User>>;
    /// Persists account links.
    async fn update_oauth_links(
        &self,
        user_id: &UserId,
        links: Vec<LinkedOAuthAccount>,
    ) -> AuthResult<()>;
    /// Updates display name and other mutable profile fields.
    async fn update_profile(
        &self,
        user_id: &UserId,
        display_name: Option<&str>,
    ) -> AuthResult<User>;
    /// Replaces the stored password hash.
    async fn set_password_hash(&self, user_id: &UserId, hash: &str) -> AuthResult<()>;
    /// Marks the user's email address as verified.
    async fn set_email_verified(&self, user_id: &UserId) -> AuthResult<()>;
    /// Replaces the user's email address (after verification).
    async fn update_email(&self, user_id: &UserId, new_email: &str) -> AuthResult<()>;
    /// Stores the TOTP secret and enables / disables MFA on the account.
    async fn set_totp(&self, user_id: &UserId, secret: Option<&str>) -> AuthResult<()>;
    /// Permanently removes the user record.
    async fn delete_user(&self, user_id: &UserId) -> AuthResult<()>;
}

/// Persistence contract for refresh-token-backed sessions.
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Saves a newly-issued session.
    async fn create_session(&self, session: Session) -> AuthResult<()>;
    /// Finds a session by refresh token id.
    async fn get_session_by_refresh_token(
        &self,
        refresh_token_id: &uuid::Uuid,
    ) -> AuthResult<Option<Session>>;
    /// Revokes the active session.
    async fn revoke_session(&self, session_id: &uuid::Uuid) -> AuthResult<()>;
    /// Returns all active (non-revoked) sessions for a user.
    async fn list_sessions_for_user(&self, user_id: &UserId) -> AuthResult<Vec<Session>>;
}

/// Persistence contract for API key issuance and validation.
#[async_trait]
pub trait ApiKeyStore: Send + Sync {
    /// Stores a generated API key hash.
    async fn create_api_key(&self, key: ApiKey) -> AuthResult<()>;
    /// Looks up an API key by id.
    async fn get_api_key(&self, id: &uuid::Uuid) -> AuthResult<Option<ApiKey>>;
    /// Revokes an API key.
    async fn revoke_api_key(&self, id: &uuid::Uuid) -> AuthResult<()>;
    /// Returns all API keys belonging to a user.
    async fn list_api_keys_for_user(&self, user_id: &UserId) -> AuthResult<Vec<ApiKey>>;
}

/// Persistence contract for role and permission checks in a tenant context.
#[async_trait]
pub trait RolesPermissionsStore: Send + Sync {
    /// Returns true when user has every required permission in the tenant.
    async fn user_has_permissions(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        required: &[String],
    ) -> AuthResult<bool>;
}

/// Persistence contract for pending account-link flows.
#[async_trait]
pub trait PendingLinkStore: Send + Sync {
    /// Creates a pending link token for a user/provider pair.
    async fn create_pending_link(
        &self,
        user_id: &UserId,
        provider: &str,
        provider_user_id: &str,
    ) -> AuthResult<String>;
    /// Consumes and resolves a pending link token.
    async fn consume_pending_link(
        &self,
        token: &str,
    ) -> AuthResult<Option<(UserId, LinkedOAuthAccount)>>;
}

/// Storage contract for auth telemetry persistence.
#[async_trait]
pub trait TelemetryStore: Send + Sync {
    /// Persists an auth domain event.
    async fn persist_event(&self, event: Event) -> AuthResult<()>;
}

/// Fanout contract for distributed SSE notifications.
#[async_trait]
pub trait SseDistributor: Send + Sync {
    /// Publishes an event for connected subscribers.
    async fn publish(&self, event: &Event) -> AuthResult<()>;
}

/// Event bus abstraction for internal pub/sub.
#[async_trait]
pub trait EventBus: Send + Sync {
    /// Publishes an event to subscribers.
    async fn publish(&self, event: Event) -> AuthResult<()>;
}

/// Persistence contract for magic-link tokens.
#[async_trait]
pub trait MagicLinkStore: Send + Sync {
    /// Stores a newly-generated magic-link token.
    async fn create_magic_link(&self, token: MagicLinkToken) -> AuthResult<()>;
    /// Consumes (deletes) a magic-link token and returns it if valid and unexpired.
    async fn consume_magic_link(&self, token: &str) -> AuthResult<Option<MagicLinkToken>>;
}

/// Persistence contract for SMS / email OTP codes.
#[async_trait]
pub trait OtpStore: Send + Sync {
    /// Stores a hashed OTP record, overwriting any existing one for the user.
    async fn create_otp(&self, record: OtpRecord) -> AuthResult<()>;
    /// Consumes the OTP record for the user, returning it if unexpired.
    async fn consume_otp(&self, user_id: &UserId) -> AuthResult<Option<OtpRecord>>;
}

/// Persistence contract for password-reset tokens.
#[async_trait]
pub trait PasswordResetStore: Send + Sync {
    /// Stores a newly-generated password-reset token.
    async fn create_password_reset(&self, token: PasswordResetToken) -> AuthResult<()>;
    /// Consumes (deletes) a password-reset token and returns it if valid and unexpired.
    async fn consume_password_reset(&self, token: &str) -> AuthResult<Option<PasswordResetToken>>;
}

/// Persistence contract for email verification / email-change tokens.
#[async_trait]
pub trait EmailVerificationStore: Send + Sync {
    /// Stores an email-verification token.
    async fn create_email_verification(&self, token: EmailVerificationToken) -> AuthResult<()>;
    /// Consumes an email-verification token and returns it if valid and unexpired.
    async fn consume_email_verification(
        &self,
        token: &str,
    ) -> AuthResult<Option<EmailVerificationToken>>;
}
