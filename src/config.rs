use std::time::Duration;

use crate::error::{AuthError, AuthResult};

/// Default mount prefix of the adapter routes, as in `awesome-node-auth`.
pub const DEFAULT_API_PREFIX: &str = "/auth";

/// Normalises a route prefix.
///
/// Surrounding whitespace and empty segments are dropped, and a leading `/`
/// is added: `"api/auth/"` becomes `"/api/auth"`. `""` and `"/"` both become
/// `""`, which mounts the routes at the root (`/ui/auth.js`). The function is
/// idempotent.
pub fn normalize_api_prefix(prefix: &str) -> String {
    prefix
        .split('/')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .fold(String::new(), |mut path, segment| {
            path.push('/');
            path.push_str(segment);
            path
        })
}

fn validate_api_prefix(prefix: &str) -> AuthResult<()> {
    let allowed = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~');
    let invalid = |segment: &&str| matches!(*segment, "." | "..") || !segment.chars().all(allowed);
    match prefix.split('/').skip(1).find(invalid) {
        Some(segment) => Err(AuthError::Config(format!(
            "api_prefix segment {segment:?} may only contain ASCII letters, digits, '-', '.', '_' and '~'"
        ))),
        None => Ok(()),
    }
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub issuer: String,
    pub audience: String,
    pub jwt_secret: String,
    pub access_token_ttl: Duration,
    pub refresh_token_ttl: Duration,
    /// Mount prefix of every adapter route: the API, the built-in UI
    /// (`<api_prefix>/ui/*`, including `auth.js` and `/ui/config`) and the
    /// admin UI (`<api_prefix>/admin`). Default `/auth`. `build()` stores it
    /// normalised (see [`normalize_api_prefix`]).
    pub api_prefix: String,
    /// Headless UI, like `ui.headless` in `awesome-node-auth`: the built-in
    /// UI pages answer 404, while `auth.js`, the other UI assets and
    /// `<api_prefix>/ui/config` are still served. Default `false`.
    pub ui_headless: bool,
    pub built_in_locales: Vec<String>,
    pub enable_idp_mode: bool,
}

impl AuthConfig {
    pub fn builder() -> AuthConfigBuilder {
        AuthConfigBuilder::default()
    }

    /// Path of the built-in UI pages: `<api_prefix>/ui`.
    pub fn auth_ui_path(&self) -> String {
        format!("{}/ui", normalize_api_prefix(&self.api_prefix))
    }

    /// Path of the browser client: `<api_prefix>/ui/auth.js`.
    pub fn auth_js_path(&self) -> String {
        format!("{}/ui/auth.js", normalize_api_prefix(&self.api_prefix))
    }

    /// Path of the embedded admin UI: `<api_prefix>/admin`.
    pub fn admin_ui_path(&self) -> String {
        format!("{}/admin", normalize_api_prefix(&self.api_prefix))
    }
}

#[derive(Debug, Clone)]
pub struct AuthConfigBuilder {
    issuer: String,
    audience: String,
    jwt_secret: String,
    access_token_ttl: Duration,
    refresh_token_ttl: Duration,
    api_prefix: String,
    ui_headless: bool,
    built_in_locales: Vec<String>,
    enable_idp_mode: bool,
}

impl Default for AuthConfigBuilder {
    fn default() -> Self {
        Self {
            issuer: "awesome-rust-auth".to_string(),
            audience: "awesome-rust-auth-clients".to_string(),
            jwt_secret: "please-change-me".to_string(),
            access_token_ttl: Duration::from_secs(15 * 60),
            refresh_token_ttl: Duration::from_secs(30 * 24 * 60 * 60),
            api_prefix: DEFAULT_API_PREFIX.to_string(),
            ui_headless: false,
            built_in_locales: vec!["en".to_string(), "it".to_string()],
            enable_idp_mode: false,
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

    /// Sets the mount prefix of every adapter route (default `/auth`).
    pub fn api_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.api_prefix = prefix.into();
        self
    }

    /// Turns the built-in UI pages off while `auth.js` stays served.
    pub fn ui_headless(mut self, headless: bool) -> Self {
        self.ui_headless = headless;
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

    pub fn build(self) -> AuthResult<AuthConfig> {
        if self.jwt_secret.len() < 16 {
            return Err(AuthError::Config(
                "jwt_secret must be at least 16 characters".to_string(),
            ));
        }
        let api_prefix = normalize_api_prefix(&self.api_prefix);
        validate_api_prefix(&api_prefix)?;

        Ok(AuthConfig {
            issuer: self.issuer,
            audience: self.audience,
            jwt_secret: self.jwt_secret,
            access_token_ttl: self.access_token_ttl,
            refresh_token_ttl: self.refresh_token_ttl,
            api_prefix,
            ui_headless: self.ui_headless,
            built_in_locales: self.built_in_locales,
            enable_idp_mode: self.enable_idp_mode,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_prefix_defaults_to_auth() {
        let config = AuthConfig::builder().build().unwrap();
        assert_eq!(config.api_prefix, "/auth");
        assert!(!config.ui_headless);
        assert_eq!(config.auth_ui_path(), "/auth/ui");
        assert_eq!(config.auth_js_path(), "/auth/ui/auth.js");
        assert_eq!(config.admin_ui_path(), "/auth/admin");
    }

    #[test]
    fn api_prefix_is_normalised() {
        for (input, expected) in [
            ("/auth", "/auth"),
            ("auth", "/auth"),
            ("/api/auth/", "/api/auth"),
            (" //api//auth// ", "/api/auth"),
            ("/", ""),
            ("", ""),
        ] {
            assert_eq!(normalize_api_prefix(input), expected, "{input:?}");
            assert_eq!(normalize_api_prefix(expected), expected, "{input:?}");
            let config = AuthConfig::builder().api_prefix(input).build().unwrap();
            assert_eq!(config.api_prefix, expected, "{input:?}");
        }
    }

    #[test]
    fn derived_paths_follow_api_prefix() {
        let config = AuthConfig::builder()
            .api_prefix("/api/auth")
            .build()
            .unwrap();
        assert_eq!(config.auth_ui_path(), "/api/auth/ui");
        assert_eq!(config.auth_js_path(), "/api/auth/ui/auth.js");
        assert_eq!(config.admin_ui_path(), "/api/auth/admin");

        let root = AuthConfig::builder().api_prefix("/").build().unwrap();
        assert_eq!(root.auth_js_path(), "/ui/auth.js");
    }

    #[test]
    fn api_prefix_rejects_unsafe_characters() {
        for input in [
            "/auth?x=1",
            "/a b",
            "/auth\"",
            "/<script>",
            "/auth#",
            "/../auth",
        ] {
            let result = AuthConfig::builder().api_prefix(input).build();
            assert!(
                matches!(result, Err(AuthError::Config(_))),
                "{input:?} should be rejected"
            );
        }
    }
}
