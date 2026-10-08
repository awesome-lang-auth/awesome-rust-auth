use warp::filters::BoxedFilter;
use warp::http::StatusCode;
use warp::{Filter, Reply};

use crate::{
    config::{AuthConfig, DEFAULT_API_PREFIX, normalize_api_prefix},
    ui,
};

/// Routes under the default prefix `/auth`, with the UI pages on.
pub fn routes() -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    build(DEFAULT_API_PREFIX, false)
}

/// Routes under `config.api_prefix`; `config.ui_headless` turns the UI pages
/// off while `auth.js` and `<prefix>/ui/config` stay served.
pub fn routes_with_config(
    config: &AuthConfig,
) -> impl Filter<Extract = impl Reply + use<>, Error = warp::Rejection> + Clone + use<> {
    build(
        &normalize_api_prefix(&config.api_prefix),
        config.ui_headless,
    )
}

/// Matches the prefix segments one by one (`""` matches nothing, i.e. the root).
fn prefix_filter(prefix: &str) -> BoxedFilter<()> {
    prefix
        .split('/')
        .filter(|segment| !segment.is_empty())
        .fold(warp::any().boxed(), |filter, segment| {
            filter.and(warp::path(segment.to_owned())).boxed()
        })
}

fn not_found() -> warp::reply::Response {
    warp::reply::with_status("not found", StatusCode::NOT_FOUND).into_response()
}

fn build(
    prefix: &str,
    headless: bool,
) -> impl Filter<Extract = impl Reply + use<>, Error = warp::Rejection> + Clone + use<> {
    let prefixed = prefix_filter(prefix);
    let ui_config_json = ui::auth_ui_config(prefix, headless).to_string();
    let admin_html = ui::render_admin_html_with_prefix(prefix);

    let health = warp::path("health").and(warp::get()).map(|| "ok");
    let admin = prefixed
        .clone()
        .and(warp::path("admin"))
        .and(warp::path::end())
        .and(warp::get())
        .map(move || warp::reply::html(admin_html.clone()));
    let admin_assets = prefixed
        .clone()
        .and(warp::path("admin"))
        .and(warp::path("assets"))
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::get())
        .map(|asset: String| match asset.as_str() {
            "admin.css" => {
                warp::reply::with_header(ui::ADMIN_CSS, "content-type", "text/css; charset=utf-8")
                    .into_response()
            }
            "admin.js" => {
                warp::reply::with_header(ui::ADMIN_JS, "content-type", "application/javascript")
                    .into_response()
            }
            _ => not_found(),
        });
    let auth = prefixed
        .clone()
        .and(warp::path("ui"))
        .and(warp::path::end())
        .and(warp::get())
        .map(move || {
            if headless {
                return not_found();
            }
            warp::reply::html(ui::AUTH_LOGIN_HTML).into_response()
        });
    let auth_ui_config = prefixed
        .clone()
        .and(warp::path("ui"))
        .and(warp::path("config"))
        .and(warp::path::end())
        .and(warp::get())
        .map(move || {
            warp::reply::with_header(ui_config_json.clone(), "content-type", "application/json")
        });
    let auth_ui_tail = prefixed
        .and(warp::path("ui"))
        .and(warp::path::tail())
        .and(warp::get())
        .map(move |tail: warp::path::Tail| {
            let path = tail.as_str().trim_matches('/');
            if let Some((content_type, content)) = ui::auth_ui_asset(path) {
                return warp::reply::with_header(content, "content-type", content_type)
                    .into_response();
            }
            if let Some(page) = ui::auth_ui_page(path).filter(|_| !headless) {
                return warp::reply::html(page).into_response();
            }
            not_found()
        });

    health
        .or(admin)
        .or(admin_assets)
        .or(auth_ui_config)
        .or(auth)
        .or(auth_ui_tail)
}

#[cfg(test)]
mod tests {
    use warp::http::StatusCode;
    use warp::{Filter, Reply, test::request};

    use super::{routes, routes_with_config};
    use crate::{config::AuthConfig, ui};

    fn configured(
        prefix: &str,
        headless: bool,
    ) -> impl Filter<Extract = impl Reply + use<>, Error = warp::Rejection> + Clone + use<> {
        let config = AuthConfig::builder()
            .api_prefix(prefix)
            .ui_headless(headless)
            .build()
            .unwrap();
        routes_with_config(&config)
    }

    async fn get<F>(filter: &F, uri: &str) -> (StatusCode, Option<String>, String)
    where
        F: Filter + 'static,
        F::Extract: Reply + Send,
    {
        let response = request().method("GET").path(uri).reply(filter).await;
        let content_type = response
            .headers()
            .get("content-type")
            .map(|value| value.to_str().unwrap().to_owned());
        (
            response.status(),
            content_type,
            String::from_utf8(response.body().to_vec()).unwrap(),
        )
    }

    async fn assert_default_routes<F>(filter: F)
    where
        F: Filter + 'static,
        F::Extract: Reply + Send,
    {
        let (status, content_type, body) = get(&filter, "/auth/ui/auth.js").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(content_type.as_deref(), Some("application/javascript"));
        assert_eq!(body, ui::AUTH_JS);

        let (status, content_type, body) = get(&filter, "/auth/ui/config").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(content_type.as_deref(), Some("application/json"));
        let config: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(config, ui::auth_ui_config("/auth", false));

        for page in ["/auth/ui", "/auth/ui/login"] {
            let (status, _, body) = get(&filter, page).await;
            assert_eq!(status, StatusCode::OK, "{page}");
            assert_eq!(body, ui::AUTH_LOGIN_HTML, "{page}");
        }

        let (status, _, body) = get(&filter, "/auth/admin").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, ui::render_admin_html());
        let (status, _, body) = get(&filter, "/auth/admin/assets/admin.css").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, ui::ADMIN_CSS);

        let (status, _, body) = get(&filter, "/health").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "ok");
    }

    #[tokio::test]
    async fn default_prefix_serves_auth_js_and_node_shaped_config() {
        assert_default_routes(routes()).await;
        assert_default_routes(configured("/auth", false)).await;
    }

    #[tokio::test]
    async fn custom_prefix_moves_every_route() {
        let filter = configured("/api/auth/", false);

        let (status, _, body) = get(&filter, "/api/auth/ui/auth.js").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, ui::AUTH_JS);

        let (status, _, body) = get(&filter, "/api/auth/ui/config").await;
        assert_eq!(status, StatusCode::OK);
        let config: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(config["apiPrefix"], "/api/auth");
        assert_eq!(config["headless"], false);

        for page in ["/api/auth/ui", "/api/auth/ui/login"] {
            let (status, _, body) = get(&filter, page).await;
            assert_eq!(status, StatusCode::OK, "{page}");
            assert_eq!(body, ui::AUTH_LOGIN_HTML, "{page}");
        }

        let (status, _, body) = get(&filter, "/api/auth/admin").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, ui::render_admin_html_with_prefix("/api/auth"));
        let (status, _, _) = get(&filter, "/api/auth/admin/assets/admin.js").await;
        assert_eq!(status, StatusCode::OK);
        let (status, _, _) = get(&filter, "/health").await;
        assert_eq!(status, StatusCode::OK);

        for old in [
            "/auth/ui/auth.js",
            "/auth/ui/config",
            "/auth/ui/login",
            "/auth/admin",
        ] {
            let (status, _, _) = get(&filter, old).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{old}");
        }
    }

    #[tokio::test]
    async fn headless_serves_auth_js_but_not_the_pages() {
        let filter = configured("/auth", true);

        for asset in ["auth.js", "base.css", "ui-i18n-keys.json"] {
            let (status, _, body) = get(&filter, &format!("/auth/ui/{asset}")).await;
            assert_eq!(status, StatusCode::OK, "{asset}");
            assert_eq!(body, ui::auth_ui_asset(asset).unwrap().1, "{asset}");
        }

        let (status, _, body) = get(&filter, "/auth/ui/config").await;
        assert_eq!(status, StatusCode::OK);
        let config: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(config["apiPrefix"], "/auth");
        assert_eq!(config["headless"], true);

        for page in ["/auth/ui", "/auth/ui/login", "/auth/ui/register"] {
            let (status, _, _) = get(&filter, page).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{page}");
        }
    }
}
