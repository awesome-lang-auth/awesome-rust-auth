# awesome-rust-auth — Full Reference

> **Quick-start README** → [README.md](./README.md)

![crates.io](https://img.shields.io/crates/v/awesome-rust-auth)
![license](https://img.shields.io/github/license/awesome-lang-auth/awesome-rust-auth)
![github stars](https://img.shields.io/github/stars/awesome-lang-auth/awesome-rust-auth)

A production-ready, **database-agnostic** JWT authentication library for Rust, inspired by [awesome-node-auth](https://github.com/awesome-lang-auth/awesome-node-auth) and compatible with its REST contract.

It delivers a 360-degree authentication and access-control layer that is completely decoupled from any specific framework or database through a simple trait-based interface pattern.

## Installation

```toml
[dependencies]
# Core only (no framework adapter)
awesome-rust-auth = "1.9.0"

# With Axum integration
awesome-rust-auth = { version = "1.9.0", features = ["axum"] }

# With Actix-web integration
awesome-rust-auth = { version = "1.9.0", features = ["actix"] }

# With Warp integration
awesome-rust-auth = { version = "1.9.0", features = ["warp"] }
```

## Quick Start

```rust
use std::sync::Arc;
use awesome_rust_auth::{AuthConfig, AuthService};

// 1. Build configuration
let config = AuthConfig::builder()
    .issuer("https://auth.example.com")
    .audience("my-app")
    .jwt_secret("replace-with-a-long-secret-32chars")
    .build()?;

// 2. Wire your store implementations (see "Database Integration" below)
let service = AuthService::new(
    config,
    Arc::new(my_user_store),
    Arc::new(my_session_store),
    Arc::new(my_telemetry_store),
    Arc::new(my_event_bus),
);

// 3. Use the service in your handlers
let (access, refresh) = service.login(login_input).await?;
# Ok::<(), awesome_rust_auth::AuthError>(())
```

## Features

- 🔐 **JWT Authentication** — Access & refresh token pair with bearer tokens or secure cookies
- 📄 **Stateful Sessions** — Hybrid JWT + store validation with real-time revocation
- 🔎 **Email/Password Auth** — Argon2 password hashing with validator-based request validation
- 🔄 **OAuth Account Linking** — Link/unlink multiple OAuth providers via pending-link flow
- 🪄 **Magic Links** — Passwordless email login; single-use 15-minute tokens
- 📱 **SMS OTP** — Phone/email one-time codes (6-digit, 10-minute TTL, SHA-256 hashed)
- 🔑 **TOTP 2FA** — Time-based OTP via `totp-rs`, compatible with Google Authenticator and Authy
- 🗃️ **Database Agnostic** — Implement traits from `src/traits.rs` for any database
- 🧩 **Trait Pattern** — Plug in only the auth features your application needs
- 🔒 **CSRF Protection** — Stateless HMAC-SHA256 double-submit cookie pattern
- 🗡️ **Roles & Permissions (RBAC)** — Tenant-aware permission checks via `RolesPermissionsStore`
- 🞢 **Multi-Tenancy** — All domain models carry a `TenantId` for full isolation
- 🗑️ **Account Management** — Profile update, password change/reset, email verification/change, delete
- 📧 **Dynamic Email Templates** — Handlebars engine with built-in `en`/`it` locale templates
- 📡 **Event-Driven Tools** — `EventBus`, telemetry, SSE, outbound/inbound webhooks
- 🔑 **API Keys (M2M)** — SHA-256-hashed keys with scopes, IP allowlist, audit events
- 📖 **OpenAPI / Swagger** — Auto-generated specs via `utoipa`
- ☠️ **Admin UI** — Embedded admin runtime at `/auth/admin` (`admin.js`, `admin.css`)
- 🎨 **Built-in Auth UI** — Zero-dependency HTML/CSS/JS pages at `/auth/ui`
- 🦝 **Webhooks** — HMAC-signed outbound webhooks + sandboxed Wasmtime inbound actions
- 🔏 **OIDC / IdP Mode** — Discovery, JWKS, and ID-token issuance for acting as an identity provider
- 🚫 **No `unsafe`** — Enforced crate-wide via `#![deny(unsafe_code)]`

---

## Database Integration — Implementing the Store Traits

The crate is completely database-agnostic. Implement the traits from `src/traits.rs` for your database and inject them via `Arc<dyn Trait>`.

### Core Store Traits

#### `UserStore` — required for all authentication flows

```rust
use async_trait::async_trait;
use awesome_rust_auth::{
    AuthResult,
    models::{LinkedOAuthAccount, TenantId, User, UserId},
    traits::UserStore,
};

pub struct MyUserStore { /* db connection */ }

#[async_trait]
impl UserStore for MyUserStore {
    // ---- Required: core CRUD ---------------------------------------------------

    /// Create a new user (signup flow).
    async fn create_user(&self, user: User) -> AuthResult<User> { todo!() }

    /// Find a user by tenant + email (login, magic-link, OTP, password reset).
    async fn get_user_by_email(
        &self, tenant_id: &TenantId, email: &str,
    ) -> AuthResult<Option<User>> { todo!() }

    /// Find a user by primary key (token refresh, 2FA, profile, …).
    async fn get_user_by_id(&self, user_id: &UserId) -> AuthResult<Option<User>> { todo!() }

    // ---- Required: field updates -----------------------------------------------

    /// Persist OAuth provider links on the user record.
    async fn update_oauth_links(
        &self, user_id: &UserId, links: Vec<LinkedOAuthAccount>,
    ) -> AuthResult<()> { todo!() }

    /// Update mutable profile fields (display name, …).
    async fn update_profile(
        &self, user_id: &UserId, display_name: Option<&str>,
    ) -> AuthResult<User> { todo!() }

    /// Replace the stored Argon2 password hash.
    async fn set_password_hash(&self, user_id: &UserId, hash: &str) -> AuthResult<()> { todo!() }

    /// Mark the user's email address as verified.
    async fn set_email_verified(&self, user_id: &UserId) -> AuthResult<()> { todo!() }

    /// Replace the user's email address (after email-change verification).
    async fn update_email(&self, user_id: &UserId, new_email: &str) -> AuthResult<()> { todo!() }

    /// Store or clear the TOTP secret and toggle the MFA flag.
    async fn set_totp(&self, user_id: &UserId, secret: Option<&str>) -> AuthResult<()> { todo!() }

    /// Permanently remove the user record.
    async fn delete_user(&self, user_id: &UserId) -> AuthResult<()> { todo!() }
}
```

#### `SessionStore` — required for login, refresh, session listing

```rust
use async_trait::async_trait;
use uuid::Uuid;
use awesome_rust_auth::{AuthResult, models::{Session, UserId}, traits::SessionStore};

#[async_trait]
impl SessionStore for MySessionStore {
    async fn create_session(&self, session: Session) -> AuthResult<()> { todo!() }
    async fn get_session_by_refresh_token(&self, id: &Uuid) -> AuthResult<Option<Session>> { todo!() }
    async fn revoke_session(&self, session_id: &Uuid) -> AuthResult<()> { todo!() }
    async fn list_sessions_for_user(&self, user_id: &UserId) -> AuthResult<Vec<Session>> { todo!() }
}
```

#### `TelemetryStore` — required; receives every auth domain event

```rust
use async_trait::async_trait;
use awesome_rust_auth::{AuthResult, models::Event, traits::TelemetryStore};

#[async_trait]
impl TelemetryStore for MyTelemetryStore {
    async fn persist_event(&self, event: Event) -> AuthResult<()> { todo!() }
}
```

#### `EventBus` — required; in-process pub/sub fanout

```rust
use async_trait::async_trait;
use awesome_rust_auth::{AuthResult, models::Event, traits::EventBus};

#[async_trait]
impl EventBus for MyEventBus {
    async fn publish(&self, event: Event) -> AuthResult<()> { todo!() }
}
```

#### `ApiKeyStore` — required for API-key (M2M) features

```rust
use async_trait::async_trait;
use uuid::Uuid;
use awesome_rust_auth::{AuthResult, models::{ApiKey, UserId}, traits::ApiKeyStore};

#[async_trait]
impl ApiKeyStore for MyApiKeyStore {
    async fn create_api_key(&self, key: ApiKey) -> AuthResult<()> { todo!() }
    async fn get_api_key(&self, id: &Uuid) -> AuthResult<Option<ApiKey>> { todo!() }
    async fn revoke_api_key(&self, id: &Uuid) -> AuthResult<()> { todo!() }
    async fn list_api_keys_for_user(&self, user_id: &UserId) -> AuthResult<Vec<ApiKey>> { todo!() }
}
```

#### `RolesPermissionsStore` — required for RBAC

```rust
use async_trait::async_trait;
use awesome_rust_auth::{AuthResult, models::{TenantId, UserId}, traits::RolesPermissionsStore};

#[async_trait]
impl RolesPermissionsStore for MyRbacStore {
    async fn user_has_permissions(
        &self, tenant_id: &TenantId, user_id: &UserId, required: &[String],
    ) -> AuthResult<bool> { todo!() }
}
```

#### `PendingLinkStore` — required for OAuth account-linking

```rust
use async_trait::async_trait;
use awesome_rust_auth::{
    AuthResult, models::{LinkedOAuthAccount, UserId}, traits::PendingLinkStore,
};

#[async_trait]
impl PendingLinkStore for MyPendingLinkStore {
    async fn create_pending_link(
        &self, user_id: &UserId, provider: &str, provider_user_id: &str,
    ) -> AuthResult<String> { todo!() }

    async fn consume_pending_link(
        &self, token: &str,
    ) -> AuthResult<Option<(UserId, LinkedOAuthAccount)>> { todo!() }
}
```

#### `MagicLinkStore` — required for magic-link auth

```rust
use async_trait::async_trait;
use awesome_rust_auth::{AuthResult, models::MagicLinkToken, traits::MagicLinkStore};

#[async_trait]
impl MagicLinkStore for MyMagicLinkStore {
    async fn create_magic_link(&self, token: MagicLinkToken) -> AuthResult<()> { todo!() }
    async fn consume_magic_link(&self, token: &str) -> AuthResult<Option<MagicLinkToken>> { todo!() }
}
```

#### `OtpStore` — required for SMS/email OTP auth

```rust
use async_trait::async_trait;
use awesome_rust_auth::{AuthResult, models::{OtpRecord, UserId}, traits::OtpStore};

#[async_trait]
impl OtpStore for MyOtpStore {
    async fn create_otp(&self, record: OtpRecord) -> AuthResult<()> { todo!() }
    async fn consume_otp(&self, user_id: &UserId) -> AuthResult<Option<OtpRecord>> { todo!() }
}
```

#### `PasswordResetStore` and `EmailVerificationStore` — required for account management

```rust
use async_trait::async_trait;
use awesome_rust_auth::{
    AuthResult,
    models::{EmailVerificationToken, PasswordResetToken},
    traits::{EmailVerificationStore, PasswordResetStore},
};

#[async_trait]
impl PasswordResetStore for MyPasswordResetStore {
    async fn create_password_reset(&self, token: PasswordResetToken) -> AuthResult<()> { todo!() }
    async fn consume_password_reset(&self, token: &str) -> AuthResult<Option<PasswordResetToken>> { todo!() }
}

#[async_trait]
impl EmailVerificationStore for MyEmailVerificationStore {
    async fn create_email_verification(&self, token: EmailVerificationToken) -> AuthResult<()> { todo!() }
    async fn consume_email_verification(&self, token: &str) -> AuthResult<Option<EmailVerificationToken>> { todo!() }
}
```

#### `SseDistributor` — optional; SSE fanout for connected clients

```rust
use async_trait::async_trait;
use awesome_rust_auth::{AuthResult, models::Event, traits::SseDistributor};

#[async_trait]
impl SseDistributor for MySseDistributor {
    async fn publish(&self, event: &Event) -> AuthResult<()> { todo!() }
}
```

---

## Configuration (`AuthConfig`)

All configurable fields with their defaults:

```rust
use std::time::Duration;
use awesome_rust_auth::AuthConfig;

let config = AuthConfig::builder()
    // JWT issuer claim (default: "awesome-rust-auth")
    .issuer("https://auth.example.com")
    // JWT audience claim (default: "awesome-rust-auth-clients")
    .audience("my-app")
    // HMAC-SHA256 secret — must be ≥16 characters
    .jwt_secret("replace-with-a-long-random-secret")
    // Access token TTL (default: 15 minutes)
    .access_token_ttl(Duration::from_secs(15 * 60))
    // Refresh token TTL (default: 30 days)
    .refresh_token_ttl(Duration::from_secs(30 * 24 * 60 * 60))
    // Path prefix for the embedded admin UI (default: "/auth/admin")
    .admin_ui_path("/auth/admin")
    // Path prefix for the built-in auth UI (default: "/auth/ui")
    .auth_ui_path("/auth/ui")
    // Path to the auth.js runtime (default: "/auth/ui/auth.js")
    .auth_js_path("/auth/ui/auth.js")
    // Locales for built-in email templates (default: ["en", "it"])
    .built_in_locales(vec!["en".to_string(), "it".to_string()])
    // Enable OIDC identity-provider endpoints (default: false)
    .enable_idp_mode(true)
    .build()?;
# Ok::<(), awesome_rust_auth::AuthError>(())
```

### JWT Claims Structure

Every access and refresh token carries the following claims:

| Claim | Type   | Description                         |
|-------|--------|-------------------------------------|
| `sub` | string | User UUID                           |
| `tid` | string | Tenant identifier                   |
| `sid` | string | Session UUID (access) or refresh-token UUID (refresh) |
| `exp` | i64    | Expiration Unix timestamp           |
| `iat` | i64    | Issued-at Unix timestamp            |
| `iss` | string | Issuer (`config.issuer`)            |
| `aud` | string | Audience (`config.audience`)        |

---

## `AuthService` — Core Authentication

```rust
use std::sync::Arc;
use awesome_rust_auth::{
    AuthConfig, AuthService,
    models::{LoginInput, SignupInput, TenantId, UserId},
};

let svc = AuthService::new(config, Arc::new(users), Arc::new(sessions),
                           Arc::new(telemetry), Arc::new(events));
```

### Signup

```rust
use awesome_rust_auth::models::SignupInput;

let user = svc.signup(SignupInput {
    email: "alice@example.com".into(),
    password: "s3cur3P@ssw0rd".into(),
    tenant_id: "tenant-abc".into(),
}).await?;
```

Password is hashed with **Argon2id** using a random 16-byte salt. Emits `EventType::Signup`.

### Login

```rust
use awesome_rust_auth::models::LoginInput;

let (access, refresh) = svc.login(LoginInput {
    email: "alice@example.com".into(),
    password: "s3cur3P@ssw0rd".into(),
    tenant_id: "tenant-abc".into(),
}).await?;

// access.token  — short-lived JWT (default: 15 min)
// refresh.token — long-lived JWT  (default: 30 days)
// refresh.token_id — Uuid for revocation
```

Emits `EventType::Login`.

### Token rotation (refresh)

```rust
let (new_access, new_refresh) = svc.rotate_refresh_token(&refresh.token).await?;
```

The old session is revoked atomically before the new session is created. Emits `EventType::Refresh`.

### Logout (revoke)

```rust
svc.revoke_refresh_token(refresh.token_id).await?;
```

Emits `EventType::Logout`.

### Session listing

```rust
let sessions: Vec<Session> = svc.list_sessions(&user_id).await?;
```

Returns all active (non-revoked) sessions for the user.

### RBAC — permission checks

```rust
svc.require_permissions(
    &rbac_store,
    &TenantId("tenant-abc".into()),
    &user_id,
    &["posts:write".to_string(), "comments:delete".to_string()],
).await?; // Err(AuthError::PermissionDenied) if not authorized
```

Emits `EventType::PermissionDenied` on failure.

### OAuth account linking (two-step)

```rust
use awesome_rust_auth::models::{CompleteLinkInput, InitiateLinkInput};

// Step 1: create pending token
let pending_token = svc.initiate_account_link(&link_store, InitiateLinkInput {
    user_id: user.id.clone(),
    tenant_id: user.tenant_id.clone(),
    provider: "github".into(),
    provider_user_id: "gh-user-12345".into(),
}).await?;

// Step 2: consume token after OAuth callback
let updated_user = svc.complete_account_link(&link_store, CompleteLinkInput {
    pending_token,
}).await?;
```

### OAuth account unlinking

```rust
use awesome_rust_auth::models::UnlinkAccountInput;

let updated_user = svc.unlink_account(UnlinkAccountInput {
    user_id: user.id.clone(),
    tenant_id: user.tenant_id.clone(),
    provider: "github".into(),
}).await?;
```

---

## `AccountService` — Account Management

```rust
use std::sync::Arc;
use awesome_rust_auth::AccountService;

let account_svc = AccountService::new(
    config.clone(),
    Arc::new(users),
    Arc::new(telemetry),
    Arc::new(events),
    Arc::new(password_reset_store),
    Arc::new(email_verification_store),
);
```

### Profile update

```rust
use awesome_rust_auth::models::UpdateProfileInput;

let updated = account_svc.update_profile(UpdateProfileInput {
    user_id: user.id.clone(),
    tenant_id: user.tenant_id.clone(),
    display_name: Some("Alice Smith".into()),
}).await?;
```

Emits `EventType::ProfileUpdated`.

### Password change (authenticated)

```rust
use awesome_rust_auth::models::ChangePasswordInput;

account_svc.change_password(ChangePasswordInput {
    user_id: user.id.clone(),
    tenant_id: user.tenant_id.clone(),
    current_password: "old-password".into(),
    new_password: "new-s3cur3-p@ss".into(),
}).await?;
```

Verifies the current password before hashing and storing the new one. Emits `EventType::PasswordReset`.

### Password reset (two-step, unauthenticated)

```rust
use awesome_rust_auth::models::{CompletePasswordResetInput, RequestPasswordResetInput};

// Step 1: generate + store token (send it in an email via MailTemplateEngine)
let token = account_svc.request_password_reset(RequestPasswordResetInput {
    email: "alice@example.com".into(),
    tenant_id: "tenant-abc".into(),
}).await?; // Err(AuthError::NotFound) if email not found

// Step 2: consume token + set new password
account_svc.complete_password_reset(CompletePasswordResetInput {
    token,
    new_password: "brand-new-p@ss".into(),
}).await?;
```

Token TTL: **1 hour**. Emits `EventType::PasswordResetRequested` and `EventType::PasswordReset`.

### Email verification

```rust
// Generate + store token (embed in verification link email)
let token = account_svc.request_email_verification(user.id.clone(), user.tenant_id.clone()).await?;

// Verify when user clicks the link
account_svc.verify_email(&token).await?;
```

Token TTL: **24 hours**. Emits `EventType::EmailVerificationRequested` and `EventType::EmailVerified`.

### Email change (two-step)

```rust
// Step 1: generate token with new email (send link to new address)
let token = account_svc.request_email_change(
    user.id.clone(),
    user.tenant_id.clone(),
    "new@example.com".into(),
).await?;

// Step 2: confirm when user clicks link
account_svc.confirm_email_change(&token).await?;
```

Token TTL: **24 hours**. Emits `EventType::EmailChanged`.

### Account deletion

```rust
account_svc.delete_account(user.id.clone(), user.tenant_id.clone()).await?;
```

Calls `UserStore::delete_user`. Emits `EventType::AccountDeleted`.

---

## `MagicLinkService` — Passwordless Email Login

```rust
use std::sync::Arc;
use awesome_rust_auth::MagicLinkService;

let ml_svc = MagicLinkService::new(
    config.clone(),
    Arc::new(users),
    Arc::new(sessions),
    Arc::new(telemetry),
    Arc::new(events),
    Arc::new(magic_link_store),
);
```

```rust
use awesome_rust_auth::models::RequestMagicLinkInput;

// Step 1: generate + store token (15-min TTL)
let token = ml_svc.request(RequestMagicLinkInput {
    email: "alice@example.com".into(),
    tenant_id: "tenant-abc".into(),
}).await?; // Err(AuthError::NotFound) if user does not exist

// Step 2: verify token from URL → issue access + refresh pair
let (access, refresh) = ml_svc.verify(&token).await?;
```

Token TTL: **15 minutes**; single-use (consumed on verify). Emits `EventType::MagicLinkRequested` and `EventType::Login`.

---

## `OtpService` — SMS / Email One-Time Passwords

```rust
use std::sync::Arc;
use awesome_rust_auth::OtpService;

let otp_svc = OtpService::new(
    config.clone(),
    Arc::new(users),
    Arc::new(sessions),
    Arc::new(telemetry),
    Arc::new(events),
    Arc::new(otp_store),
);
```

```rust
use awesome_rust_auth::models::{RequestOtpInput, VerifyOtpInput};

// Step 1: generate 6-digit code + store SHA-256 hash
let plaintext_code = otp_svc.request(RequestOtpInput {
    email: "alice@example.com".into(),
    tenant_id: "tenant-abc".into(),
}).await?; // deliver plaintext_code via SMS / email

// Step 2: verify code → issue access + refresh pair
let (access, refresh) = otp_svc.verify(VerifyOtpInput {
    email: "alice@example.com".into(),
    tenant_id: "tenant-abc".into(),
    code: plaintext_code,
}).await?;
```

Code TTL: **10 minutes**; single-use. Emits `EventType::OtpRequested` and `EventType::Login`.

---

## TOTP 2FA — `totp_service`

```rust
use awesome_rust_auth::totp_service::{setup_totp, verify_totp_code};

// Step 1: generate secret + otpauth:// URI (show QR code to user)
let setup = setup_totp("alice@example.com", "MyApp")?;
// setup.secret — base32-encoded; persist via UserStore::set_totp after confirmation
// setup.uri    — otpauth:// URL for QR rendering

// Step 2: verify first code from authenticator app, then persist
if verify_totp_code(&setup.secret, &submitted_code)? {
    user_store.set_totp(&user_id, Some(&setup.secret)).await?;
}

// On every MFA login: verify current code
let ok = verify_totp_code(&stored_secret, &submitted_code)?;
```

Algorithm: **SHA-1**, 6 digits, 30-second step, ±1 step window for clock skew.

---

## `ApiKeyManager` — Machine-to-Machine API Keys

```rust
use std::sync::Arc;
use awesome_rust_auth::ApiKeyManager;

let api_keys = ApiKeyManager::new(Arc::new(api_key_store));
```

### Issue a key

```rust
use awesome_rust_auth::models::IssueApiKeyInput;

let issued = api_keys.issue(IssueApiKeyInput {
    user_id: user.id.clone(),
    tenant_id: user.tenant_id.clone(),
    label: "CI deploy key".into(),
    scopes: vec!["deploy:write".into()],
    ip_allowlist: vec!["192.168.1.0/24".into()],
}).await?;

// issued.plaintext_key — shown ONCE; format: "<uuid>.<hex-key>"
// Store issued.id for revocation.
```

### Authenticate an incoming API key

```rust
// Client sends: Authorization: ApiKey <uuid>.<hex-key>
let key_meta = api_keys.authenticate(&plaintext_key_from_header).await?;
// key_meta.scopes, key_meta.ip_allowlist, key_meta.user_id, …
```

### Revoke / list

```rust
api_keys.revoke(&key_id).await?;

let all_keys: Vec<ApiKey> = api_keys.list_for_user(&user_id).await?;
```

Key storage: SHA-256 of `<uuid>:<raw-hex>`. The plaintext key is never stored.

---

## CSRF Protection — `csrf`

Stateless double-submit cookie pattern. No server-side state required.

```rust
use std::time::Duration;
use awesome_rust_auth::csrf::{generate_csrf_token, validate_csrf_token};

let ttl = Duration::from_secs(3600);

// On form render: generate token, set as cookie + embed in form/header
let csrf = generate_csrf_token(&jwt_secret, ttl)?;
// csrf.token   — "<nonce_hex>.<timestamp>.<hmac_hex>"
// csrf.expires_at

// On form submit: validate cookie value == header/body value + HMAC
validate_csrf_token(&jwt_secret, &submitted_token, ttl)?;
// Err(AuthError::InvalidToken) if tampered or expired
```

Token format: `<16-byte-nonce-hex>.<unix-timestamp>.<hmac-sha256-hex>`.

---

## Email Templating — `mail::MailTemplateEngine`

Handlebars-powered engine with built-in `en` and `it` locale templates.

### Built-in templates

| Template name       | Aliases                     | Description              |
|---------------------|-----------------------------|--------------------------|
| `welcome`           | —                           | New-user welcome email   |
| `password_reset`    | `password-reset`            | Password reset link      |
| `magic_link`        | `magic-link`                | Magic-link login         |
| `verify_email`      | `verify-email`              | Email verification link  |
| `email_changed`     | `email-changed`             | Email address changed    |
| `invitation`        | —                           | User invitation          |

### Usage

```rust
use awesome_rust_auth::mail::MailTemplateEngine;

let engine = MailTemplateEngine::with_builtin_locales()?;

// Render with automatic fallback to "en" if locale is missing
let html = engine.render("it", "magic_link", &serde_json::json!({
    "magic_link_url": "https://example.com/auth/magic-link?token=abc",
    "user_name": "Alice"
}))?;
```

### Register custom locale/template at runtime

```rust
let mut engine = MailTemplateEngine::with_builtin_locales()?;

engine.register_locale_template(
    "fr",
    "welcome",
    "<h1>Bienvenue, {{user_name}}!</h1>",
)?;
```

If a locale is not found, the engine falls back to `"en"`.

---

## OIDC / IdP Mode — `oidc`

When `enable_idp_mode(true)` is set, the `oidc` module provides:

```rust
use awesome_rust_auth::oidc::{discovery, issue_id_token, jwks};

// Discovery document at /.well-known/openid-configuration
let doc = discovery(&config, "https://auth.example.com");
// doc.authorization_endpoint, doc.token_endpoint, doc.jwks_uri, …

// JWKS endpoint
let key_set = jwks(&config);

// Issue an OIDC ID token
let id_token = issue_id_token(
    &config,
    &user_id,
    &tenant_id,
    Some("alice@example.com"),
    Some(true), // email_verified
    3600,       // TTL in seconds
)?;
```

Signing algorithm: **HS256** (same key as access tokens).

---

## Webhooks — `webhook`

### Outbound webhook signing / verification

```rust
use awesome_rust_auth::webhook::{sign_webhook, verify_webhook};

let payload = serde_json::to_vec(&event).unwrap();

// Sign (include X-Signature header in outbound request)
let sig = sign_webhook("webhook-secret", &payload)?;
// sig: hex-encoded HMAC-SHA256

// Verify on the receiving end
let ok = verify_webhook("webhook-secret", &received_body, &received_sig_header)?;
```

### Inbound action sandboxing (Wasmtime)

Inbound webhook handlers can be executed in a resource-limited Wasmtime sandbox:

```rust
use awesome_rust_auth::webhook::run_inbound_action_wasm;

let wasm_bytes: &[u8] = /* load from DB/file */;
run_inbound_action_wasm(wasm_bytes)?;
// Limits: 8 MB memory, 1 instance, 100 000 fuel units
```

---

## Event Bus & Telemetry

Every service method emits a domain event through both `TelemetryStore::persist_event` and `EventBus::publish`.

### Event types

| `EventType`                  | Emitted by                                     |
|------------------------------|------------------------------------------------|
| `Signup`                     | `AuthService::signup`                          |
| `Login`                      | `AuthService::login`, magic-link/OTP verify    |
| `Refresh`                    | `AuthService::rotate_refresh_token`            |
| `Logout`                     | `AuthService::revoke_refresh_token`            |
| `Failure`                    | (reserved for future use)                      |
| `MagicLinkRequested`         | `MagicLinkService::request`                    |
| `OtpRequested`               | `OtpService::request`                          |
| `MfaEnabled`                 | (reserved)                                     |
| `MfaDisabled`                | (reserved)                                     |
| `ProfileUpdated`             | `AccountService::update_profile`               |
| `PasswordResetRequested`     | `AccountService::request_password_reset`       |
| `PasswordReset`              | `AccountService::complete_password_reset` / `change_password` |
| `EmailVerificationRequested` | `AccountService::request_email_verification` / `request_email_change` |
| `EmailVerified`              | `AccountService::verify_email`                 |
| `EmailChanged`               | `AccountService::confirm_email_change`         |
| `AccountDeleted`             | `AccountService::delete_account`               |
| `AccountLinked`              | `AuthService::initiate_account_link` / `complete_account_link` |
| `AccountUnlinked`            | `AuthService::unlink_account`                  |
| `ApiKeyCreated`              | (reserved)                                     |
| `ApiKeyRevoked`              | (reserved)                                     |
| `PermissionDenied`           | `AuthService::require_permissions`             |

---

## Built-in UI — `ui`

The crate ships a ready-to-serve authentication UI and admin panel as embedded static assets.

### Auth UI pages (served at `/auth/ui/`)

| Path                 | Description                                   |
|----------------------|-----------------------------------------------|
| `/auth/ui/`          | Login page (default)                          |
| `/auth/ui/register`  | Registration page                             |
| `/auth/ui/forgot-password` | Forgot password page                    |
| `/auth/ui/reset-password`  | Reset password page                     |
| `/auth/ui/magic-link`      | Magic-link confirmation page            |
| `/auth/ui/2fa`             | 2FA challenge page                      |
| `/auth/ui/verify-email`    | Email verification page                 |
| `/auth/ui/account-conflict`| OAuth account-conflict resolution page  |
| `/auth/ui/link-verify`     | Account-link verification page          |

### Auth UI assets

| Path                        | MIME type          |
|-----------------------------|--------------------|
| `/auth/ui/auth.js`          | `application/javascript` |
| `/auth/ui/base.css`         | `text/css`         |
| `/auth/ui/ui-i18n-keys.json`| `application/json` |

### Self-configuring `/auth/ui/config` endpoint

The UI bootstraps by fetching `/auth/ui/config` at startup. The default config JSON is available at `ui::AUTH_UI_CONFIG_JSON` and includes:

```json
{
  "apiPrefix": "/auth",
  "features": {
    "register": false,
    "magicLink": false,
    "sms": false,
    "google": false,
    "github": false,
    "forgotPassword": false,
    "verifyEmail": false,
    "twoFactor": false
  },
  "ui": {
    "primaryColor": "#4a90d9",
    "secondaryColor": "#6c757d",
    "siteName": "Awesome Node Auth"
  },
  "translations": {},
  "lang": "en",
  "headless": false
}
```

Set `"headless": true` to disable the built-in UI pages and use the `auth.js` runtime only (SPA mode).

### Admin UI (served at `/auth/admin/`)

The admin panel is injected via `ui::render_admin_html()` and bootstraps from `window.__ADMIN_CONFIG__`:

```json
{
  "base": "/auth/admin",
  "featSessions": false,
  "featRoles": false,
  "featTenants": false,
  "featMetadata": false,
  "feat2faPolicy": false,
  "featControl": false,
  "featLinkedAccounts": false,
  "featApiKeys": true,
  "featWebhooks": true,
  "featTemplates": true
}
```

Use the `ui` module helpers directly in your framework handler:

```rust
use awesome_rust_auth::ui::{auth_ui_asset, auth_ui_page, render_admin_html};

// Serve a UI page
if let Some(html) = auth_ui_page("login") {
    // respond with html, content-type: text/html
}

// Serve a static asset
if let Some((content_type, content)) = auth_ui_asset("auth.js") {
    // respond with content, content-type
}

// Serve the admin panel
let admin_html = render_admin_html();
```

---

## OpenAPI / Swagger — `openapi`

```rust
use awesome_rust_auth::openapi::AuthApiDoc;
use utoipa::OpenApi;

let spec = AuthApiDoc::openapi();
// Serialize to JSON/YAML and serve at /auth/openapi.json
let json = spec.to_json()?;
```

The spec includes `POST /auth/signup` and `POST /auth/login` by default. Extend `AuthApiDoc` in your application to add framework-specific endpoints.

---

## Framework Adapters

### Axum (feature = `"axum"`)

```rust
#[cfg(feature = "axum")]
{
    let router = awesome_rust_auth::adapters::axum::router();
    // mount under your axum Router
}
```

### Actix-web (feature = `"actix"`)

```rust
#[cfg(feature = "actix")]
{
    let scope = awesome_rust_auth::adapters::actix::scope();
    // mount under your actix-web App
}
```

### Warp (feature = `"warp"`)

```rust
#[cfg(feature = "warp")]
{
    let routes = awesome_rust_auth::adapters::warp::routes();
    // compose with your warp filters
}
```

---

## Error Handling — `AuthError`

All service methods return `AuthResult<T> = Result<T, AuthError>`.

| Variant              | Meaning                                              |
|----------------------|------------------------------------------------------|
| `InvalidCredentials` | Wrong email/password or API key                      |
| `Validation(msg)`    | Input validation failed (email format, length, …)    |
| `NotFound`           | User, session, or token not found                    |
| `InvalidToken`       | JWT or opaque token is malformed, expired, or tampered |
| `RevokedToken`       | Refresh token has already been revoked               |
| `TenantMismatch`     | Token tenant does not match the request tenant       |
| `PermissionDenied`   | RBAC check failed                                    |
| `Config(msg)`        | Configuration error (e.g., jwt_secret too short)     |
| `Storage(msg)`       | Persistence layer returned an error                  |
| `Crypto(msg)`        | Cryptographic operation failed                       |
| `Sandbox(msg)`       | Wasmtime sandbox error                               |

---

## Parity Snapshot vs `awesome-node-auth`

| Capability | Status | Notes |
|---|---|---|
| Auth strategies (email/password, magic link, SMS OTP, TOTP 2FA, OAuth linking) | ✅ Implemented | Core services cover all flows |
| Token management (access/refresh rotation, revocation) | ✅ Implemented | `AuthService` provides full issuance + rotation + revocation |
| CSRF protection | ✅ Implemented | Stateless HMAC-SHA256 double-submit cookie pattern |
| Stateful sessions | ✅ Implemented | Session creation, rotation, revocation, and listing via `SessionStore` |
| Identity Provider (IdP) mode | ✅ Implemented | OIDC discovery, JWKS, and ID-token issuance |
| Dynamic email templates + UI i18n fallback | ✅ Implemented | `en`/`it` built-in; runtime locale/template registration supported |
| Account management | ✅ Implemented | Profile, password change/reset, email verification/change, delete |
| Account linking (OAuth) | ✅ Implemented | Two-step pending-link + unlink flows |
| RBAC | ✅ Implemented | Tenant-aware permission checks with `PermissionDenied` event |
| Multi-tenancy | ✅ Implemented | `TenantId` propagated across all models, tokens, and services |
| Admin panel | ✅ Implemented | Embedded admin UI runtime at `/auth/admin` |
| Built-in UI + auth runtime | ✅ Implemented | Multi-page Vanilla UI at `/auth/ui` + self-configuring `/auth/ui/config` |
| Inbound/Outbound webhooks | ✅ Implemented | HMAC signing/verification + Wasmtime sandbox for inbound actions |
| Event-driven tooling | ✅ Implemented | `EventBus`, `TelemetryStore`, `SseDistributor` traits + 19 event types |
| API keys (M2M) | ✅ Implemented | `ApiKeyManager` with SHA-256 hashing, scopes, IP allowlist, revocation |
| OpenAPI / Swagger docs | ✅ Implemented | `utoipa`-based `AuthApiDoc` |

---

## Examples

| Example | Framework | Database |
|---------|-----------|----------|
| `examples/axum-postgres` | Axum | PostgreSQL |
| `examples/actix-mongodb` | Actix-web | MongoDB |
| `examples/warp-sqlite` | Warp | SQLite |

Run an example:

```bash
cargo run --example axum-postgres --features axum
```

---

## API Compatibility Target

This crate targets the same REST shape and token conventions as:

- `ng-awesome-node-auth`
- `awesome-node-auth-flutter`

Explicit known deviations (also available via `api_contract::compatibility_notes()`):

1. OAuth provider-specific payload shape follows the Rust service contract and can be adapted per integration layer.
2. Inbound webhook action decorators run through the built-in Wasmtime execution model.

---

## Crate Design Principles

- **Database-agnostic** persistence via traits in `src/traits.rs`
- **Framework-agnostic** core in `src/service.rs`
- **No `unsafe`** — enforced via `#![deny(unsafe_code)]`
- **Semver-stable** public API evolution from `1.9.0`
- All external I/O is async (`async-trait`)
- Input validation via `validator` derive macros
