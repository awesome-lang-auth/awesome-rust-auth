# awesome-rust-auth

`awesome-rust-auth` is a framework-agnostic Rust authentication crate inspired by [awesome-node-auth](https://www.awesomenodeauth.com).

## Parity Snapshot vs `awesome-node-auth`

| Capability | Status in `awesome-rust-auth` (core crate scope) | Notes |
|---|---|---|
| Auth strategies (email/password, magic link, SMS OTP, TOTP 2FA, OAuth linking) | ✅ Implemented | Core services cover email/password auth, magic-link, SMS OTP/TOTP helpers, and account-linking flows. |
| Token management (cookie/bearer, access/refresh rotation, secure cookies) | ✅ Implemented | `AuthService` provides access/refresh issuance with rotation/revocation and supports CSRF primitives for secure browser integration. |
| Identity Provider (IdP) mode (OIDC discovery, authorization, token, userinfo, JWKS) | ✅ Implemented | OIDC discovery/JWKS support and ID token issuance are available in core modules for IdP integrations. |
| Stateful sessions | ✅ Implemented | Session creation, rotation, revocation, and listing are implemented through `SessionStore` + `AuthService`. |
| Dynamic email templates + UI i18n fallback | ✅ Implemented | Built-in localized templates (`welcome`, `password_reset`/`password-reset`, `magic_link`/`magic-link`, `verify_email`/`verify-email`, `email_changed`/`email-changed`, `invitation`) are bundled for `en`/`it`, with runtime locale/template registration hooks. |
| CSRF protection | ✅ Implemented | Stateless CSRF token generation and verification helpers are implemented in core. |
| Account management | ✅ Implemented | `AccountService` provides profile, password, reset, verification, email-change, and delete-account flows. |
| Account linking | ✅ Implemented | Pending-link creation/consumption and unlink flows are implemented in `AuthService`. |
| RBAC | ✅ Implemented | `RolesPermissionsStore` + `AuthService::require_permissions` provide tenant-aware authorization checks with permission-denied event telemetry. |
| Multi-tenancy | ✅ Implemented | Tenant-aware IDs/models are propagated across contracts, tokens, and core services. |
| Admin panel | ✅ Implemented | `/auth/admin` ships an embedded admin UI runtime (`admin.css`/`admin.js`) with configurable client bootstrap (`window.__ADMIN_CONFIG__`). |
| Built-in UI + auth runtime (`auth.js`) | ✅ Implemented | `/auth/ui` ships the same multi-page Vanilla UI asset set and runtime contract used by `awesome-node-auth`, including `/auth/ui/config`. |
| Inbound/Outbound webhooks | ✅ Implemented | Webhook contracts and execution helpers are available via `webhook` module + event-bus integrations. |
| Event-driven tooling (event bus, SSE, inbound/outbound webhooks, telemetry) | ✅ Implemented | Event bus, SSE distributor contract, webhook execution helpers, and telemetry/event persistence contracts are available. |
| API keys (M2M) | ✅ Implemented | API-key issuing, hashing, authentication, listing, and revocation are implemented in `ApiKeyManager`. |
| OpenAPI / Swagger docs | ✅ Implemented | OpenAPI schemas and auth endpoint documentation are shipped via `utoipa` support in `openapi`/`api_contract` modules. |

> `✅ Implemented` in this table indicates framework-agnostic core crate capabilities.
> Transport-specific middleware and full HTTP integration remain adapter/application concerns.

## Features

- Access/refresh JWT pair with rotation + revocation primitives
- Email/password auth (`argon2`) with validator-based request validation
- OAuth account-link model scaffolding
- Tenant-aware domain model + RBAC store contract
- Session, API key, telemetry, SSE, event-bus, and webhook traits
- Embedded `awesome-node-auth`-compatible Admin/Auth UI runtimes (`/auth/admin`, `/auth/ui`) with `admin.js`, `auth.js`, and `/auth/ui/config`
- Mail templating via Handlebars with built-in `welcome`, `password_reset`, `magic_link`, `verify_email`, `email_changed`, and `invitation` locale templates (`en`, `it`) plus custom registration
- OIDC discovery + JWKS helpers for IDP mode
- OpenAPI generation via `utoipa`
- Adapter modules behind feature flags for Axum, Actix-web, and Warp

## Installation

```toml
[dependencies]
awesome-rust-auth = { version = "1.9.0", features = ["axum"] }
```

## Configuration

```rust
use awesome_rust_auth::AuthConfig;

let config = AuthConfig::builder()
    .issuer("https://auth.example.com")
    .audience("my-app")
    .jwt_secret("replace-with-a-long-secret")
    .enable_idp_mode(true)
    .build()?;
# Ok::<(), awesome_rust_auth::AuthError>(())
```

## Adapter usage

### Axum

```rust
#[cfg(feature = "axum")]
{
    let app = awesome_rust_auth::adapters::axum::router();
    let _ = app;
}
```

### Actix-web

```rust
#[cfg(feature = "actix")]
{
    let scope = awesome_rust_auth::adapters::actix::scope();
    let _ = scope;
}
```

### Warp

```rust
#[cfg(feature = "warp")]
{
    let routes = awesome_rust_auth::adapters::warp::routes();
    let _ = routes;
}
```

## API compatibility target

This crate targets the same REST shape and token conventions used by:

- [`@awesome-lang-auth/angular`](https://github.com/awesome-lang-auth/awesome-angular-auth)
- [`awesome_flutter_auth`](https://github.com/awesome-lang-auth/awesome-flutter-auth)

Current explicit deviations (also available via `compatibility_notes()`):

1. OAuth provider-specific payload shape follows the core Rust service contract and can be adapted per integration layer.
2. Inbound webhook action decorators run through the built-in secure Wasmtime execution model.

## Examples

- `examples/axum-postgres`
- `examples/actix-mongodb`
- `examples/warp-sqlite`

## Crate design principles

- Database-agnostic persistence via traits in `src/traits.rs`
- Framework-agnostic core in `src/service.rs`
- No `unsafe`
- Public API designed for semver-stable evolution from `0.1.0`
