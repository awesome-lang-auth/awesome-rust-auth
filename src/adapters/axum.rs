use axum::{
    Json, Router,
    extract::Path,
    http::{StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};

use crate::ui;

async fn admin_ui() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        ui::render_admin_html(),
    )
}

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

async fn auth_ui_root() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        ui::AUTH_LOGIN_HTML,
    )
}

async fn auth_ui_tail(Path(tail): Path<String>) -> impl IntoResponse {
    if let Some((content_type, content)) = ui::auth_ui_asset(tail.trim_matches('/')) {
        return ([(header::CONTENT_TYPE, content_type)], content).into_response();
    }
    if let Some(page) = ui::auth_ui_page(tail.trim_matches('/')) {
        return ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], page).into_response();
    }
    StatusCode::NOT_FOUND.into_response()
}

async fn auth_ui_config() -> Json<serde_json::Value> {
    Json(
        serde_json::from_str(ui::AUTH_UI_CONFIG_JSON)
            .expect("AUTH_UI_CONFIG_JSON should be valid JSON"),
    )
}

async fn health() -> &'static str {
    "ok"
}

pub fn router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/auth/admin", get(admin_ui))
        .route("/auth/admin/assets/{asset}", get(admin_asset))
        .route("/auth/ui", get(auth_ui_root))
        .route("/auth/ui/", get(auth_ui_root))
        .route("/auth/ui/config", get(auth_ui_config))
        .route("/auth/ui/{*tail}", get(auth_ui_tail))
        .route("/auth/login", post(health))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header},
    };
    use tower::ServiceExt;

    use super::router;
    use crate::ui;

    async fn get(uri: &str) -> (StatusCode, Option<String>, String) {
        let response = router()
            .oneshot(Request::get(uri).body(Body::empty()).unwrap())
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
}
