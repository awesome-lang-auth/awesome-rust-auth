use std::time::Duration;

use crate::error::{AuthError, AuthResult};

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub issuer: String,
    pub audience: String,
    pub jwt_secret: String,
    pub access_token_ttl: Duration,
    pub refresh_token_ttl: Duration,
    pub admin_ui_path: String,
    pub auth_ui_path: String,
    pub auth_js_path: String,
    pub built_in_locales: Vec<String>,
    pub enable_idp_mode: bool,
    /// When `true`, [`AuthService::register`](crate::AuthService::register)
    /// also opens a session for the new account, exactly as
    /// [`AuthService::login`](crate::AuthService::login) does (same token
    /// pair, same session row, same `Login` event). Default `false`: register
    /// only creates the account and the client logs in afterwards.
    pub issue_session_on_register: bool,
}

impl AuthConfig {
    pub fn builder() -> AuthConfigBuilder {
        AuthConfigBuilder::default()
    }
}

#[derive(Debug, Clone)]
pub struct AuthConfigBuilder {
    issuer: String,
    audience: String,
    jwt_secret: String,
    access_token_ttl: Duration,
    refresh_token_ttl: Duration,
    admin_ui_path: String,
    auth_ui_path: String,
    auth_js_path: String,
    built_in_locales: Vec<String>,
    enable_idp_mode: bool,
    issue_session_on_register: bool,
}

impl Default for AuthConfigBuilder {
    fn default() -> Self {
        Self {
            issuer: "awesome-rust-auth".to_string(),
            audience: "awesome-rust-auth-clients".to_string(),
            jwt_secret: "please-change-me".to_string(),
            access_token_ttl: Duration::from_secs(15 * 60),
            refresh_token_ttl: Duration::from_secs(30 * 24 * 60 * 60),
            admin_ui_path: "/auth/admin".to_string(),
            auth_ui_path: "/auth/ui".to_string(),
            auth_js_path: "/auth/ui/auth.js".to_string(),
            built_in_locales: vec!["en".to_string(), "it".to_string()],
            enable_idp_mode: false,
            issue_session_on_register: false,
        }
    }
}

impl AuthConfigBuilder {
    pub fn issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = issuer.into();
        self
    }

    pub fn audience(mut self, audience: impl Into<String>) -> Self {
        self.audience = audience.into();
        self
    }

    pub fn jwt_secret(mut self, secret: impl Into<String>) -> Self {
        self.jwt_secret = secret.into();
        self
    }

    pub fn access_token_ttl(mut self, ttl: Duration) -> Self {
        self.access_token_ttl = ttl;
        self
    }

    pub fn refresh_token_ttl(mut self, ttl: Duration) -> Self {
        self.refresh_token_ttl = ttl;
        self
    }

    pub fn admin_ui_path(mut self, path: impl Into<String>) -> Self {
        self.admin_ui_path = path.into();
        self
    }

    pub fn auth_ui_path(mut self, path: impl Into<String>) -> Self {
        self.auth_ui_path = path.into();
        self
    }

    pub fn auth_js_path(mut self, path: impl Into<String>) -> Self {
        self.auth_js_path = path.into();
        self
    }

    pub fn built_in_locales(mut self, locales: Vec<String>) -> Self {
        self.built_in_locales = locales;
        self
    }

    pub fn enable_idp_mode(mut self, enabled: bool) -> Self {
        self.enable_idp_mode = enabled;
        self
    }

    /// Lets [`AuthService::register`](crate::AuthService::register) open a
    /// session for the new account, the way login does (default: `false`).
    pub fn issue_session_on_register(mut self, enabled: bool) -> Self {
        self.issue_session_on_register = enabled;
        self
    }

    pub fn build(self) -> AuthResult<AuthConfig> {
        if self.jwt_secret.len() < 16 {
            return Err(AuthError::Config(
                "jwt_secret must be at least 16 characters".to_string(),
            ));
        }

        Ok(AuthConfig {
            issuer: self.issuer,
            audience: self.audience,
            jwt_secret: self.jwt_secret,
            access_token_ttl: self.access_token_ttl,
            refresh_token_ttl: self.refresh_token_ttl,
            admin_ui_path: self.admin_ui_path,
            auth_ui_path: self.auth_ui_path,
            auth_js_path: self.auth_js_path,
            built_in_locales: self.built_in_locales,
            enable_idp_mode: self.enable_idp_mode,
            issue_session_on_register: self.issue_session_on_register,
        })
    }
}
