use std::sync::Arc;

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand::RngCore;
use time::{Duration as TimeDuration, OffsetDateTime};
use uuid::Uuid;
use validator::Validate;

use crate::{
    config::AuthConfig,
    error::{AuthError, AuthResult},
    models::{
        AccessToken, CompleteLinkInput, Event, EventType, InitiateLinkInput, LinkedOAuthAccount,
        LoginInput, RefreshToken, Session, SignupInput, TenantId, UnlinkAccountInput, User, UserId,
    },
    traits::{
        EventBus, PendingLinkStore, RolesPermissionsStore, SessionStore, TelemetryStore, UserStore,
    },
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Claims {
    sub: String,
    tid: String,
    sid: String,
    exp: i64,
    iat: i64,
    iss: String,
    aud: String,
}

/// Result of [`AuthService::register`].
#[derive(Debug, Clone)]
pub struct RegisterOutcome {
    /// The account that was created.
    pub user: User,
    /// The token pair [`AuthService::login`] would return, when
    /// [`AuthConfig::issue_session_on_register`] is on; `None` otherwise.
    pub session: Option<(AccessToken, RefreshToken)>,
}

/// Core authentication service (email/password + token management).
#[derive(Clone)]
pub struct AuthService<U, S, T, E>
where
    U: UserStore,
    S: SessionStore,
    T: TelemetryStore,
    E: EventBus,
{
    pub(crate) config: AuthConfig,
    users: Arc<U>,
    sessions: Arc<S>,
    telemetry: Arc<T>,
    events: Arc<E>,
}

/// A lightweight token-minting helper that holds only the config.
///
/// Used by the magic-link and OTP service modules to mint tokens without
/// duplicating the JWT logic.
pub struct TokenMinter {
    config: AuthConfig,
}

impl TokenMinter {
    pub fn mint_access_token(
        &self,
        user_id: &UserId,
        tenant_id: &TenantId,
        session_id: &Uuid,
    ) -> AuthResult<AccessToken> {
        issue_token(
            &self.config,
            user_id,
            tenant_id,
            session_id,
            self.config.access_token_ttl.as_secs() as i64,
        )
    }

    pub fn mint_refresh_token(
        &self,
        user_id: &UserId,
        tenant_id: &TenantId,
        refresh_token_id: &Uuid,
    ) -> AuthResult<RefreshToken> {
        issue_refresh_token(
            &self.config,
            user_id,
            tenant_id,
            refresh_token_id,
            self.config.refresh_token_ttl.as_secs() as i64,
        )
    }
}

impl<U, S, T, E> AuthService<U, S, T, E>
where
    U: UserStore,
    S: SessionStore,
    T: TelemetryStore,
    E: EventBus,
{
    pub fn new(
        config: AuthConfig,
        users: Arc<U>,
        sessions: Arc<S>,
        telemetry: Arc<T>,
        events: Arc<E>,
    ) -> Self {
        Self {
            config,
            users,
            sessions,
            telemetry,
            events,
        }
    }

    /// Returns a [`TokenMinter`] backed by this service's config.
    ///
    /// Used by auxiliary services (magic link, OTP) that need to mint tokens
    /// without holding the full `AuthService`.
    pub fn bare(config: &AuthConfig) -> TokenMinter {
        TokenMinter {
            config: config.clone(),
        }
    }

    pub async fn signup(&self, input: SignupInput) -> AuthResult<User> {
        input
            .validate()
            .map_err(|err| AuthError::Validation(err.to_string()))?;

        let tenant_id = TenantId(input.tenant_id.clone());
        if self
            .users
            .get_user_by_email(&tenant_id, &input.email)
            .await?
            .is_some()
        {
            return Err(AuthError::Validation("email already exists".to_string()));
        }

        let salt = {
            let mut bytes = [0_u8; 16];
            rand::thread_rng().fill_bytes(&mut bytes);
            bytes
        };
        let salt = argon2::password_hash::SaltString::encode_b64(&salt)
            .map_err(|err| AuthError::Crypto(err.to_string()))?;
        let password_hash = Argon2::default()
            .hash_password(input.password.as_bytes(), &salt)
            .map_err(|err| AuthError::Crypto(err.to_string()))?
            .to_string();

        let user = User {
            id: UserId(Uuid::new_v4()),
            tenant_id,
            email: input.email,
            password_hash: Some(password_hash),
            oauth_accounts: Vec::new(),
            is_mfa_enabled: false,
            totp_secret: None,
            is_email_verified: false,
            display_name: None,
            created_at: OffsetDateTime::now_utc(),
        };

        let created = self.users.create_user(user).await?;
        self.emit_event(
            EventType::Signup,
            Some(created.tenant_id.clone()),
            Some(created.id.clone()),
            serde_json::json!({"email": created.email}),
        )
        .await?;
        Ok(created)
    }

    pub async fn login(&self, input: LoginInput) -> AuthResult<(AccessToken, RefreshToken)> {
        input
            .validate()
            .map_err(|err| AuthError::Validation(err.to_string()))?;

        let tenant_id = TenantId(input.tenant_id.clone());
        let user = self
            .users
            .get_user_by_email(&tenant_id, &input.email)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;
        let hash = user
            .password_hash
            .clone()
            .ok_or(AuthError::InvalidCredentials)?;

        let parsed = PasswordHash::new(&hash).map_err(|err| AuthError::Crypto(err.to_string()))?;
        Argon2::default()
            .verify_password(input.password.as_bytes(), &parsed)
            .map_err(|_| AuthError::InvalidCredentials)?;

        self.open_session(&user).await
    }

    /// Creates an account like [`signup`](Self::signup) and, when
    /// [`AuthConfig::issue_session_on_register`] is on, also opens a session
    /// for it exactly as [`login`](Self::login) does: same token pair, same
    /// session row, same `Login` event. With the option off (the default) no
    /// session is opened and the client logs in afterwards.
    ///
    /// A refused registration (invalid input, duplicate email, store error)
    /// returns the error and opens nothing.
    ///
    /// This crate's login does not gate on `is_email_verified`. An application
    /// that refuses login to unverified accounts must leave this option off,
    /// or register would bypass its verification gate.
    pub async fn register(&self, input: SignupInput) -> AuthResult<RegisterOutcome> {
        let user = self.signup(input).await?;
        let session = if self.config.issue_session_on_register {
            Some(self.open_session(&user).await?)
        } else {
            None
        };
        Ok(RegisterOutcome { user, session })
    }

    /// Opens a session for an authenticated user: session row, token pair,
    /// `Login` event. Shared by [`login`](Self::login) and
    /// [`register`](Self::register).
    async fn open_session(&self, user: &User) -> AuthResult<(AccessToken, RefreshToken)> {
        let now = OffsetDateTime::now_utc();
        let session = Session {
            id: Uuid::new_v4(),
            user_id: user.id.clone(),
            tenant_id: user.tenant_id.clone(),
            refresh_token_id: Uuid::new_v4(),
            expires_at: now + TimeDuration::seconds(self.config.refresh_token_ttl.as_secs() as i64),
            revoked_at: None,
        };
        self.sessions.create_session(session.clone()).await?;

        let access = self.mint_access_token(&user.id, &user.tenant_id, &session.id)?;
        let refresh =
            self.mint_refresh_token(&user.id, &user.tenant_id, &session.refresh_token_id)?;

        self.emit_event(
            EventType::Login,
            Some(user.tenant_id.clone()),
            Some(user.id.clone()),
            serde_json::json!({"session_id": session.id}),
        )
        .await?;
        Ok((access, refresh))
    }

    pub async fn rotate_refresh_token(
        &self,
        token: &str,
    ) -> AuthResult<(AccessToken, RefreshToken)> {
        let claims = self.decode_claims(token)?;
        let refresh_id = Uuid::parse_str(&claims.sid).map_err(|_| AuthError::InvalidToken)?;
        let existing = self
            .sessions
            .get_session_by_refresh_token(&refresh_id)
            .await?
            .ok_or(AuthError::InvalidToken)?;
        if existing.revoked_at.is_some() {
            return Err(AuthError::RevokedToken);
        }

        self.sessions.revoke_session(&existing.id).await?;

        let new_session = Session {
            id: Uuid::new_v4(),
            user_id: existing.user_id.clone(),
            tenant_id: existing.tenant_id.clone(),
            refresh_token_id: Uuid::new_v4(),
            expires_at: OffsetDateTime::now_utc()
                + TimeDuration::seconds(self.config.refresh_token_ttl.as_secs() as i64),
            revoked_at: None,
        };
        self.sessions.create_session(new_session.clone()).await?;

        let access = self.mint_access_token(
            &new_session.user_id,
            &new_session.tenant_id,
            &new_session.id,
        )?;
        let refresh = self.mint_refresh_token(
            &new_session.user_id,
            &new_session.tenant_id,
            &new_session.refresh_token_id,
        )?;

        self.emit_event(
            EventType::Refresh,
            Some(new_session.tenant_id),
            Some(new_session.user_id),
            serde_json::json!({"old_session_id": existing.id, "new_session_id": new_session.id}),
        )
        .await?;
        Ok((access, refresh))
    }

    pub async fn revoke_refresh_token(&self, refresh_token_id: Uuid) -> AuthResult<()> {
        let session = self
            .sessions
            .get_session_by_refresh_token(&refresh_token_id)
            .await?
            .ok_or(AuthError::NotFound)?;
        self.sessions.revoke_session(&session.id).await?;
        self.emit_event(
            EventType::Logout,
            Some(session.tenant_id),
            Some(session.user_id),
            serde_json::json!({"session_id": session.id}),
        )
        .await
    }

    /// Returns all active sessions for a user.
    pub async fn list_sessions(&self, user_id: &UserId) -> AuthResult<Vec<Session>> {
        self.sessions.list_sessions_for_user(user_id).await
    }

    /// Checks whether the user has all of `required` permissions in the given
    /// tenant.  Returns `Err(AuthError::PermissionDenied)` when access is
    /// denied.
    pub async fn require_permissions<R: RolesPermissionsStore>(
        &self,
        rbac: &R,
        tenant_id: &TenantId,
        user_id: &UserId,
        required: &[String],
    ) -> AuthResult<()> {
        let allowed = rbac
            .user_has_permissions(tenant_id, user_id, required)
            .await?;
        if !allowed {
            self.emit_event(
                EventType::PermissionDenied,
                Some(tenant_id.clone()),
                Some(user_id.clone()),
                serde_json::json!({"required": required}),
            )
            .await?;
            return Err(AuthError::PermissionDenied);
        }
        Ok(())
    }

    // ── Account linking ───────────────────────────────────────────────────────

    /// Step 1: stores a pending link token for the OAuth provider pair.
    ///
    /// Returns the pending token that must be passed to the OAuth callback.
    pub async fn initiate_account_link<L: PendingLinkStore>(
        &self,
        links: &L,
        input: InitiateLinkInput,
    ) -> AuthResult<String> {
        let token = links
            .create_pending_link(&input.user_id, &input.provider, &input.provider_user_id)
            .await?;

        self.emit_event(
            EventType::AccountLinked,
            Some(input.tenant_id),
            Some(input.user_id),
            serde_json::json!({"provider": input.provider, "step": "initiated"}),
        )
        .await?;

        Ok(token)
    }

    /// Step 2: consumes the pending link token and persists the OAuth link on
    /// the user record.
    pub async fn complete_account_link<L: PendingLinkStore>(
        &self,
        links: &L,
        input: CompleteLinkInput,
    ) -> AuthResult<User> {
        let (user_id, linked_account) = links
            .consume_pending_link(&input.pending_token)
            .await?
            .ok_or(AuthError::InvalidToken)?;

        let user = self
            .users
            .get_user_by_id(&user_id)
            .await?
            .ok_or(AuthError::NotFound)?;

        let mut links_vec = user.oauth_accounts.clone();
        links_vec.push(linked_account.clone());
        self.users.update_oauth_links(&user_id, links_vec).await?;

        let updated = self
            .users
            .get_user_by_id(&user_id)
            .await?
            .ok_or(AuthError::NotFound)?;

        self.emit_event(
            EventType::AccountLinked,
            Some(user.tenant_id),
            Some(user_id),
            serde_json::json!({"provider": linked_account.provider}),
        )
        .await?;

        Ok(updated)
    }

    /// Removes an OAuth provider link from the user's account.
    pub async fn unlink_account(&self, input: UnlinkAccountInput) -> AuthResult<User> {
        let user = self
            .users
            .get_user_by_id(&input.user_id)
            .await?
            .ok_or(AuthError::NotFound)?;

        let remaining: Vec<LinkedOAuthAccount> = user
            .oauth_accounts
            .iter()
            .filter(|a| a.provider != input.provider)
            .cloned()
            .collect();

        if remaining.len() == user.oauth_accounts.len() {
            return Err(AuthError::NotFound);
        }

        self.users
            .update_oauth_links(&input.user_id, remaining)
            .await?;

        let updated = self
            .users
            .get_user_by_id(&input.user_id)
            .await?
            .ok_or(AuthError::NotFound)?;

        self.emit_event(
            EventType::AccountUnlinked,
            Some(input.tenant_id),
            Some(input.user_id),
            serde_json::json!({"provider": input.provider}),
        )
        .await?;

        Ok(updated)
    }

    // ── token minting ─────────────────────────────────────────────────────────

    pub(crate) fn mint_access_token(
        &self,
        user_id: &UserId,
        tenant_id: &TenantId,
        session_id: &Uuid,
    ) -> AuthResult<AccessToken> {
        issue_token(
            &self.config,
            user_id,
            tenant_id,
            session_id,
            self.config.access_token_ttl.as_secs() as i64,
        )
    }

    pub(crate) fn mint_refresh_token(
        &self,
        user_id: &UserId,
        tenant_id: &TenantId,
        refresh_token_id: &Uuid,
    ) -> AuthResult<RefreshToken> {
        issue_refresh_token(
            &self.config,
            user_id,
            tenant_id,
            refresh_token_id,
            self.config.refresh_token_ttl.as_secs() as i64,
        )
    }

    fn decode_claims(&self, token: &str) -> AuthResult<Claims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_audience(std::slice::from_ref(&self.config.audience));
        validation.set_issuer(std::slice::from_ref(&self.config.issuer));
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.config.jwt_secret.as_bytes()),
            &validation,
        )
        .map(|data| data.claims)
        .map_err(|_| AuthError::InvalidToken)
    }

    async fn emit_event(
        &self,
        event_type: EventType,
        tenant_id: Option<TenantId>,
        user_id: Option<UserId>,
        metadata: serde_json::Value,
    ) -> AuthResult<()> {
        let event = Event {
            event_type,
            tenant_id,
            user_id,
            metadata,
            happened_at: OffsetDateTime::now_utc(),
        };
        self.telemetry.persist_event(event.clone()).await?;
        self.events.publish(event).await
    }
}

// ── free functions shared with TokenMinter ────────────────────────────────────

fn issue_token(
    config: &AuthConfig,
    user_id: &UserId,
    tenant_id: &TenantId,
    session_id: &Uuid,
    ttl_secs: i64,
) -> AuthResult<AccessToken> {
    let now = OffsetDateTime::now_utc();
    let exp = now + TimeDuration::seconds(ttl_secs);
    let claims = Claims {
        sub: user_id.0.to_string(),
        tid: tenant_id.0.clone(),
        sid: session_id.to_string(),
        exp: exp.unix_timestamp(),
        iat: now.unix_timestamp(),
        iss: config.issuer.clone(),
        aud: config.audience.clone(),
    };
    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
    .map_err(|err| AuthError::Crypto(err.to_string()))?;

    Ok(AccessToken {
        token,
        expires_at: exp,
    })
}

fn issue_refresh_token(
    config: &AuthConfig,
    user_id: &UserId,
    tenant_id: &TenantId,
    refresh_token_id: &Uuid,
    ttl_secs: i64,
) -> AuthResult<RefreshToken> {
    let now = OffsetDateTime::now_utc();
    let exp = now + TimeDuration::seconds(ttl_secs);
    let claims = Claims {
        sub: user_id.0.to_string(),
        tid: tenant_id.0.clone(),
        sid: refresh_token_id.to_string(),
        exp: exp.unix_timestamp(),
        iat: now.unix_timestamp(),
        iss: config.issuer.clone(),
        aud: config.audience.clone(),
    };
    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
    .map_err(|err| AuthError::Crypto(err.to_string()))?;

    Ok(RefreshToken {
        token,
        token_id: *refresh_token_id,
        expires_at: exp,
    })
}
