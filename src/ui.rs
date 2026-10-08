use crate::config::{DEFAULT_API_PREFIX, normalize_api_prefix};

pub const AUTH_JS: &str = include_str!("assets/auth.js");
pub const AUTH_BASE_CSS: &str = include_str!("assets/base.css");
pub const AUTH_UI_I18N_KEYS_JSON: &str = include_str!("assets/ui-i18n-keys.json");
pub const AUTH_LOGIN_HTML: &str = include_str!("assets/login.html");
pub const AUTH_REGISTER_HTML: &str = include_str!("assets/register.html");
pub const AUTH_FORGOT_PASSWORD_HTML: &str = include_str!("assets/forgot-password.html");
pub const AUTH_RESET_PASSWORD_HTML: &str = include_str!("assets/reset-password.html");
pub const AUTH_MAGIC_LINK_HTML: &str = include_str!("assets/magic-link.html");
pub const AUTH_2FA_HTML: &str = include_str!("assets/2fa.html");
pub const AUTH_VERIFY_EMAIL_HTML: &str = include_str!("assets/verify-email.html");
pub const AUTH_ACCOUNT_CONFLICT_HTML: &str = include_str!("assets/account-conflict.html");
pub const AUTH_LINK_VERIFY_HTML: &str = include_str!("assets/link-verify.html");

pub const ADMIN_CSS: &str = include_str!("assets/admin.css");
pub const ADMIN_JS: &str = include_str!("assets/admin.js");
pub const ADMIN_HTML_TEMPLATE: &str = include_str!("assets/admin.html");

/// The `<api_prefix>/ui/config` document for the default prefix (`/auth`)
/// with the pages on. [`auth_ui_config`] builds it for any prefix.
pub const AUTH_UI_CONFIG_JSON: &str = r##"{
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
}"##;

/// The admin UI bootstrap (`window.__ADMIN_CONFIG__`) for the default prefix
/// (`/auth`). [`admin_config`] builds it for any prefix.
pub const ADMIN_CONFIG_JSON: &str = r#"{
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
  "featTemplates": true,
  "featUpload": false,
  "uploadBaseUrl": "/auth/admin/assets/uploads",
  "sessionBased": false,
  "authApiPrefix": "/auth",
  "cookiePrefix": null
}"#;

/// The `<api_prefix>/ui/config` document: [`AUTH_UI_CONFIG_JSON`] with
/// `apiPrefix` set to the (normalised) prefix and `headless` to the flag.
/// `auth.js` reads both fields.
pub fn auth_ui_config(api_prefix: &str, headless: bool) -> serde_json::Value {
    let mut config: serde_json::Value =
        serde_json::from_str(AUTH_UI_CONFIG_JSON).expect("AUTH_UI_CONFIG_JSON is valid JSON");
    config["apiPrefix"] = normalize_api_prefix(api_prefix).into();
    config["headless"] = headless.into();
    config
}

/// The admin UI bootstrap: [`ADMIN_CONFIG_JSON`] with `base`,
/// `uploadBaseUrl` and `authApiPrefix` following the (normalised) prefix.
pub fn admin_config(api_prefix: &str) -> serde_json::Value {
    let prefix = normalize_api_prefix(api_prefix);
    let mut config: serde_json::Value =
        serde_json::from_str(ADMIN_CONFIG_JSON).expect("ADMIN_CONFIG_JSON is valid JSON");
    config["base"] = format!("{prefix}/admin").into();
    config["uploadBaseUrl"] = format!("{prefix}/admin/assets/uploads").into();
    config["authApiPrefix"] = prefix.into();
    config
}

/// The admin UI page for the default prefix (`/auth/admin`).
pub fn render_admin_html() -> String {
    render_admin_html_with_prefix(DEFAULT_API_PREFIX)
}

/// The admin UI page served at `<api_prefix>/admin`.
pub fn render_admin_html_with_prefix(api_prefix: &str) -> String {
    let prefix = normalize_api_prefix(api_prefix);
    // The config is inlined in a <script>: escape `<` so it cannot close it.
    let config = admin_config(&prefix).to_string().replace('<', "\\u003c");
    ADMIN_HTML_TEMPLATE
        .replace("__ADMIN_BASE_URL__", &format!("{prefix}/admin"))
        .replace("__ADMIN_CONFIG__", &config)
}

pub fn auth_ui_asset(path: &str) -> Option<(&'static str, &'static str)> {
    match path {
        "auth.js" => Some(("application/javascript", AUTH_JS)),
        "base.css" => Some(("text/css; charset=utf-8", AUTH_BASE_CSS)),
        "auth.css" => Some(("text/css; charset=utf-8", AUTH_BASE_CSS)),
        "ui-i18n-keys.json" => Some(("application/json", AUTH_UI_I18N_KEYS_JSON)),
        _ => None,
    }
}

pub fn auth_ui_page(path: &str) -> Option<&'static str> {
    match path.trim_matches('/') {
        "" | "login" => Some(AUTH_LOGIN_HTML),
        "register" => Some(AUTH_REGISTER_HTML),
        "forgot-password" => Some(AUTH_FORGOT_PASSWORD_HTML),
        "reset-password" => Some(AUTH_RESET_PASSWORD_HTML),
        "magic-link" => Some(AUTH_MAGIC_LINK_HTML),
        "2fa" => Some(AUTH_2FA_HTML),
        "verify-email" => Some(AUTH_VERIFY_EMAIL_HTML),
        "account-conflict" => Some(AUTH_ACCOUNT_CONFLICT_HTML),
        "link-verify" => Some(AUTH_LINK_VERIFY_HTML),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ui_config_matches_the_const() {
        let expected: serde_json::Value = serde_json::from_str(AUTH_UI_CONFIG_JSON).unwrap();
        assert_eq!(auth_ui_config("/auth", false), expected);
        assert_eq!(expected["apiPrefix"], "/auth");
    }

    #[test]
    fn ui_config_follows_prefix_and_headless() {
        let config = auth_ui_config("api/auth/", true);
        assert_eq!(config["apiPrefix"], "/api/auth");
        assert_eq!(config["headless"], true);
        assert_eq!(config["features"]["register"], false);
    }

    #[test]
    fn default_admin_config_matches_the_const() {
        let expected: serde_json::Value = serde_json::from_str(ADMIN_CONFIG_JSON).unwrap();
        assert_eq!(admin_config("/auth"), expected);
    }

    #[test]
    fn admin_html_follows_prefix() {
        let html = render_admin_html_with_prefix("/api/auth");
        assert!(html.contains(r#"href="/api/auth/admin/assets/admin.css""#));
        assert!(html.contains(r#"src="/api/auth/admin/assets/admin.js""#));
        assert!(html.contains(r#""authApiPrefix":"/api/auth""#));
        assert!(html.contains(r#""base":"/api/auth/admin""#));
        assert!(!html.contains("__ADMIN_"));

        let default_html = render_admin_html();
        assert!(default_html.contains(r#"src="/auth/admin/assets/admin.js""#));
        assert!(default_html.contains(r#""authApiPrefix":"/auth""#));
    }
}
