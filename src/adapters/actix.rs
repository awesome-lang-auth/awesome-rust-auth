use actix_web::{HttpResponse, Scope, get, web};

use crate::{
    config::{AuthConfig, DEFAULT_API_PREFIX, normalize_api_prefix},
    ui,
};

/// Per-scope values that depend on the prefix and the headless switch.
struct UiState {
    headless: bool,
    ui_config_json: String,
    admin_html: String,
}

#[get("/health")]
async fn health() -> HttpResponse {
    HttpResponse::Ok().body("ok")
}

async fn admin_ui(state: web::Data<UiState>) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(state.admin_html.clone())
}

async fn admin_asset(asset: web::Path<String>) -> HttpResponse {
    match asset.into_inner().as_str() {
        "admin.css" => HttpResponse::Ok()
            .content_type("text/css; charset=utf-8")
            .body(ui::ADMIN_CSS),
        "admin.js" => HttpResponse::Ok()
            .content_type("application/javascript")
            .body(ui::ADMIN_JS),
        _ => HttpResponse::NotFound().finish(),
    }
}

async fn auth_ui_root(state: web::Data<UiState>) -> HttpResponse {
    if state.headless {
        return HttpResponse::NotFound().finish();
    }
    HttpResponse::Ok()
        .content_type("text/html")
        .body(ui::AUTH_LOGIN_HTML)
}

async fn auth_ui_tail(state: web::Data<UiState>, tail: web::Path<String>) -> HttpResponse {
    let path = tail.into_inner();
    let path = path.trim_matches('/');
    if let Some((content_type, content)) = ui::auth_ui_asset(path) {
        return HttpResponse::Ok().content_type(content_type).body(content);
    }
    if let Some(page) = ui::auth_ui_page(path).filter(|_| !state.headless) {
        return HttpResponse::Ok().content_type("text/html").body(page);
    }
    HttpResponse::NotFound().finish()
}

async fn auth_ui_config(state: web::Data<UiState>) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .body(state.ui_config_json.clone())
}

async fn login_stub() -> HttpResponse {
    HttpResponse::Ok().finish()
}

/// Routes under the default prefix `/auth`, with the UI pages on.
pub fn scope() -> Scope {
    build(DEFAULT_API_PREFIX, false)
}

/// Routes under `config.api_prefix`; `config.ui_headless` turns the UI pages
/// off while `auth.js` and `<prefix>/ui/config` stay served.
pub fn scope_with_config(config: &AuthConfig) -> Scope {
    build(
        &normalize_api_prefix(&config.api_prefix),
        config.ui_headless,
    )
}

fn build(prefix: &str, headless: bool) -> Scope {
    let state = web::Data::new(UiState {
        headless,
        ui_config_json: ui::auth_ui_config(prefix, headless).to_string(),
        admin_html: ui::render_admin_html_with_prefix(prefix),
    });

    // Registration order matters: `{tail:.*}` would also match `config`.
    web::scope("")
        .app_data(state)
        .service(health)
        .route(&format!("{prefix}/admin"), web::get().to(admin_ui))
        .route(
            &format!("{prefix}/admin/assets/{{asset}}"),
            web::get().to(admin_asset),
        )
        .route(&format!("{prefix}/ui"), web::get().to(auth_ui_root))
        .route(
            &format!("{prefix}/ui/config"),
            web::get().to(auth_ui_config),
        )
        .route(
            &format!("{prefix}/ui/{{tail:.*}}"),
            web::get().to(auth_ui_tail),
        )
        .route(&format!("{prefix}/login"), web::post().to(login_stub))
}

#[cfg(test)]
mod tests {
    use actix_web::{
        App,
        http::{StatusCode, header},
        test,
    };

    use super::{scope, scope_with_config};
    use crate::{config::AuthConfig, ui};

    async fn call(
        scope: actix_web::Scope,
        request: test::TestRequest,
    ) -> (StatusCode, Option<String>, String) {
        let app = test::init_service(App::new().service(scope)).await;
        let response = test::call_service(&app, request.to_request()).await;
        let status = response.status();
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .map(|value| value.to_str().unwrap().to_owned());
        let body = test::read_body(response).await;
        (
            status,
            content_type,
            String::from_utf8(body.to_vec()).unwrap(),
        )
    }

    async fn get(scope: actix_web::Scope, uri: &str) -> (StatusCode, Option<String>, String) {
        call(scope, test::TestRequest::get().uri(uri)).await
    }

    fn configured(prefix: &str, headless: bool) -> actix_web::Scope {
        let config = AuthConfig::builder()
            .api_prefix(prefix)
            .ui_headless(headless)
            .build()
            .unwrap();
        scope_with_config(&config)
    }

    #[actix_web::test]
    async fn default_prefix_serves_auth_js_and_node_shaped_config() {
        let defaults: [fn() -> actix_web::Scope; 2] = [scope, || configured("/auth", false)];
        for make in defaults {
            let (status, content_type, body) = get(make(), "/auth/ui/auth.js").await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(content_type.as_deref(), Some("application/javascript"));
            assert_eq!(body, ui::AUTH_JS);

            let (status, content_type, body) = get(make(), "/auth/ui/config").await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(content_type.as_deref(), Some("application/json"));
            let config: serde_json::Value = serde_json::from_str(&body).unwrap();
            assert_eq!(config, ui::auth_ui_config("/auth", false));

            for page in ["/auth/ui", "/auth/ui/", "/auth/ui/login"] {
                let (status, _, body) = get(make(), page).await;
                assert_eq!(status, StatusCode::OK, "{page}");
                assert_eq!(body, ui::AUTH_LOGIN_HTML, "{page}");
            }

            let (status, _, body) = get(make(), "/auth/admin").await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(body, ui::render_admin_html());
            let (status, _, body) = get(make(), "/auth/admin/assets/admin.css").await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(body, ui::ADMIN_CSS);

            let (status, _, body) = get(make(), "/health").await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(body, "ok");

            let (status, _, _) = call(make(), test::TestRequest::post().uri("/auth/login")).await;
            assert_eq!(status, StatusCode::OK);
        }
    }

    #[actix_web::test]
    async fn custom_prefix_moves_every_route() {
        let make = || configured("/api/auth/", false);

        let (status, _, body) = get(make(), "/api/auth/ui/auth.js").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, ui::AUTH_JS);

        let (status, _, body) = get(make(), "/api/auth/ui/config").await;
        assert_eq!(status, StatusCode::OK);
        let config: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(config["apiPrefix"], "/api/auth");
        assert_eq!(config["headless"], false);

        for page in ["/api/auth/ui", "/api/auth/ui/", "/api/auth/ui/login"] {
            let (status, _, body) = get(make(), page).await;
            assert_eq!(status, StatusCode::OK, "{page}");
            assert_eq!(body, ui::AUTH_LOGIN_HTML, "{page}");
        }

        let (status, _, body) = get(make(), "/api/auth/admin").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, ui::render_admin_html_with_prefix("/api/auth"));
        let (status, _, _) = get(make(), "/api/auth/admin/assets/admin.js").await;
        assert_eq!(status, StatusCode::OK);

        let (status, _, _) = call(make(), test::TestRequest::post().uri("/api/auth/login")).await;
        assert_eq!(status, StatusCode::OK);
        let (status, _, _) = get(make(), "/health").await;
        assert_eq!(status, StatusCode::OK);

        for old in [
            "/auth/ui/auth.js",
            "/auth/ui/config",
            "/auth/ui/login",
            "/auth/admin",
        ] {
            let (status, _, _) = get(make(), old).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{old}");
        }
    }

    #[actix_web::test]
    async fn headless_serves_auth_js_but_not_the_pages() {
        let make = || configured("/auth", true);

        for asset in ["auth.js", "base.css", "ui-i18n-keys.json"] {
            let (status, _, body) = get(make(), &format!("/auth/ui/{asset}")).await;
            assert_eq!(status, StatusCode::OK, "{asset}");
            assert_eq!(body, ui::auth_ui_asset(asset).unwrap().1, "{asset}");
        }

        let (status, _, body) = get(make(), "/auth/ui/config").await;
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
            let (status, _, _) = get(make(), page).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{page}");
        }
    }
}
