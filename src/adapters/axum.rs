use axum::{
    Json, Router,
    extract::Path,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};

use crate::{
    config::{AuthConfig, DEFAULT_API_PREFIX, normalize_api_prefix},
    ui,
};

async fn admin_asset(Path(asset): Path<String>) -> impl IntoResponse {
    match asset.as_str() {
        "admin.css" => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
            ui::ADMIN_CSS,
        )
            .into_response(),
        "admin.js" => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/javascript")],
            ui::ADMIN_JS,
        )
            .into_response(),
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

fn auth_ui_root(headless: bool) -> Response {
    if headless {
        return StatusCode::NOT_FOUND.into_response();
    }
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        ui::AUTH_LOGIN_HTML,
    )
        .into_response()
}

fn auth_ui_tail(tail: &str, headless: bool) -> Response {
    if let Some((content_type, content)) = ui::auth_ui_asset(tail.trim_matches('/')) {
        return ([(header::CONTENT_TYPE, content_type)], content).into_response();
    }
    if let Some(page) = ui::auth_ui_page(tail.trim_matches('/')).filter(|_| !headless) {
        return ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], page).into_response();
    }
    StatusCode::NOT_FOUND.into_response()
}

async fn health() -> &'static str {
    "ok"
}

/// Routes under the default prefix `/auth`, with the UI pages on.
pub fn router() -> Router {
    build(DEFAULT_API_PREFIX, false)
}

/// Routes under `config.api_prefix`; `config.ui_headless` turns the UI pages
/// off while `auth.js` and `<prefix>/ui/config` stay served.
pub fn router_with_config(config: &AuthConfig) -> Router {
    build(
        &normalize_api_prefix(&config.api_prefix),
        config.ui_headless,
    )
}

fn build(prefix: &str, headless: bool) -> Router {
    let ui_config = ui::auth_ui_config(prefix, headless);
    let admin_html = ui::render_admin_html_with_prefix(prefix);

    Router::new()
        .route("/health", get(health))
        .route(
            &format!("{prefix}/admin"),
            get(move || {
                let html = admin_html.clone();
                async move { ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], html) }
            }),
        )
        .route(
            &format!("{prefix}/admin/assets/{{asset}}"),
            get(admin_asset),
        )
        .route(
            &format!("{prefix}/ui"),
            get(move || async move { auth_ui_root(headless) }),
        )
        .route(
            &format!("{prefix}/ui/"),
            get(move || async move { auth_ui_root(headless) }),
        )
        .route(
            &format!("{prefix}/ui/config"),
            get(move || {
                let config = ui_config.clone();
                async move { Json(config) }
            }),
        )
        .route(
            &format!("{prefix}/ui/{{*tail}}"),
            get(move |Path(tail): Path<String>| async move { auth_ui_tail(&tail, headless) }),
        )
        .route(&format!("{prefix}/login"), post(health))
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::{Body, to_bytes},
        http::{Method, Request, StatusCode, header},
    };
    use tower::ServiceExt;

    use super::{router, router_with_config};
    use crate::{config::AuthConfig, ui};

    async fn send(app: Router, method: Method, uri: &str) -> (StatusCode, Option<String>, String) {
        let response = app
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .map(|value| value.to_str().unwrap().to_owned());
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (
            status,
            content_type,
            String::from_utf8(body.to_vec()).unwrap(),
        )
    }

    async fn get_from(app: Router, uri: &str) -> (StatusCode, Option<String>, String) {
        send(app, Method::GET, uri).await
    }

    async fn get(uri: &str) -> (StatusCode, Option<String>, String) {
        get_from(router(), uri).await
    }

    fn configured(prefix: &str, headless: bool) -> Router {
        let config = AuthConfig::builder()
            .api_prefix(prefix)
            .ui_headless(headless)
            .build()
            .unwrap();
        router_with_config(&config)
    }

    #[test]
    fn router_builds() {
        let _ = router();
    }

    #[tokio::test]
    async fn health_responds_ok() {
        let (status, _, body) = get("/health").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "ok");
    }

    #[tokio::test]
    async fn admin_assets_are_served() {
        for (asset, content_type, content) in [
            ("admin.css", "text/css; charset=utf-8", ui::ADMIN_CSS),
            ("admin.js", "application/javascript", ui::ADMIN_JS),
        ] {
            let (status, actual_type, body) = get(&format!("/auth/admin/assets/{asset}")).await;
            assert_eq!(status, StatusCode::OK, "{asset}");
            assert_eq!(actual_type.as_deref(), Some(content_type), "{asset}");
            assert_eq!(body, content, "{asset}");
        }

        let (status, _, _) = get("/auth/admin/assets/missing.css").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn auth_ui_assets_are_served() {
        for asset in ["auth.js", "base.css", "auth.css", "ui-i18n-keys.json"] {
            let (content_type, content) = ui::auth_ui_asset(asset).unwrap();
            let (status, actual_type, body) = get(&format!("/auth/ui/{asset}")).await;
            assert_eq!(status, StatusCode::OK, "{asset}");
            assert_eq!(actual_type.as_deref(), Some(content_type), "{asset}");
            assert_eq!(body, content, "{asset}");
        }

        let (status, _, _) = get("/auth/ui/missing.js").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn auth_ui_pages_and_config_are_served() {
        let (status, content_type, body) = get("/auth/ui/login").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(content_type.as_deref(), Some("text/html; charset=utf-8"));
        assert_eq!(body, ui::AUTH_LOGIN_HTML);

        let (status, content_type, _) = get("/auth/ui/config").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(content_type.as_deref(), Some("application/json"));
    }

    #[tokio::test]
    async fn default_prefix_serves_auth_js_and_node_shaped_config() {
        for app in [router(), configured("/auth", false)] {
            let (status, _, body) = get_from(app.clone(), "/auth/ui/auth.js").await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(body, ui::AUTH_JS);

            let (status, _, body) = get_from(app.clone(), "/auth/ui/config").await;
            assert_eq!(status, StatusCode::OK);
            let config: serde_json::Value = serde_json::from_str(&body).unwrap();
            assert_eq!(config, ui::auth_ui_config("/auth", false));

            let (status, _, body) = get_from(app.clone(), "/auth/ui").await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(body, ui::AUTH_LOGIN_HTML);

            let (status, _, body) = get_from(app.clone(), "/auth/admin").await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(body, ui::render_admin_html());

            let (status, _, _) = send(app, Method::POST, "/auth/login").await;
            assert_eq!(status, StatusCode::OK);
        }
    }

    #[tokio::test]
    async fn custom_prefix_moves_every_route() {
        let app = configured("/api/auth/", false);

        let (status, content_type, body) = get_from(app.clone(), "/api/auth/ui/auth.js").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(content_type.as_deref(), Some("application/javascript"));
        assert_eq!(body, ui::AUTH_JS);

        let (status, _, body) = get_from(app.clone(), "/api/auth/ui/config").await;
        assert_eq!(status, StatusCode::OK);
        let config: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(config["apiPrefix"], "/api/auth");
        assert_eq!(config["headless"], false);

        for page in ["/api/auth/ui", "/api/auth/ui/", "/api/auth/ui/login"] {
            let (status, _, body) = get_from(app.clone(), page).await;
            assert_eq!(status, StatusCode::OK, "{page}");
            assert_eq!(body, ui::AUTH_LOGIN_HTML, "{page}");
        }

        let (status, _, body) = get_from(app.clone(), "/api/auth/admin").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, ui::render_admin_html_with_prefix("/api/auth"));
        let (status, _, _) = get_from(app.clone(), "/api/auth/admin/assets/admin.js").await;
        assert_eq!(status, StatusCode::OK);

        let (status, _, _) = send(app.clone(), Method::POST, "/api/auth/login").await;
        assert_eq!(status, StatusCode::OK);
        let (status, _, _) = get_from(app.clone(), "/health").await;
        assert_eq!(status, StatusCode::OK);

        for old in [
            "/auth/ui/auth.js",
            "/auth/ui/config",
            "/auth/ui/login",
            "/auth/admin",
        ] {
            let (status, _, _) = get_from(app.clone(), old).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{old}");
        }
    }

    #[tokio::test]
    async fn root_prefix_mounts_at_the_root() {
        let app = configured("/", false);

        let (status, _, body) = get_from(app.clone(), "/ui/auth.js").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, ui::AUTH_JS);

        let (status, _, body) = get_from(app, "/ui/config").await;
        assert_eq!(status, StatusCode::OK);
        let config: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(config["apiPrefix"], "");
    }

    #[tokio::test]
    async fn headless_serves_auth_js_but_not_the_pages() {
        let app = configured("/auth", true);

        for asset in ["auth.js", "base.css", "ui-i18n-keys.json"] {
            let (status, _, body) = get_from(app.clone(), &format!("/auth/ui/{asset}")).await;
            assert_eq!(status, StatusCode::OK, "{asset}");
            assert_eq!(body, ui::auth_ui_asset(asset).unwrap().1, "{asset}");
        }

        let (status, _, body) = get_from(app.clone(), "/auth/ui/config").await;
        assert_eq!(status, StatusCode::OK);
        let config: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(config["apiPrefix"], "/auth");
        assert_eq!(config["headless"], true);

        for page in [
            "/auth/ui",
            "/auth/ui/",
            "/auth/ui/login",
            "/auth/ui/register",
        ] {
            let (status, _, _) = get_from(app.clone(), page).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{page}");
        }
    }
}
