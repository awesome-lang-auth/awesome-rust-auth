use crate::{AuthConfig, api_contract::compatibility_notes, mail::MailTemplateEngine};

#[test]
fn config_builder_rejects_short_secret() {
    let result = AuthConfig::builder().jwt_secret("short").build();
    assert!(result.is_err());
}

#[test]
fn compatibility_notes_include_deviations() {
    let notes = compatibility_notes();
    assert!(!notes.known_deviations.is_empty());
    assert!(
        notes.known_deviations.iter().any(|note| note
            .contains("OAuth provider-specific payload shape follows the Rust service contract")),
        "compatibility notes should mention OAuth provider payload deviation"
    );
    assert!(
        notes
            .known_deviations
            .iter()
            .any(|note| note.contains("Inbound webhook action decorators are executed through the built-in Wasmtime sandbox model")),
        "compatibility notes should mention webhook action decorator deviation"
    );
}

#[test]
fn english_template_renders() {
    let engine = MailTemplateEngine::with_builtin_locales().expect("engine should initialize");
    let rendered = engine
        .render(
            "en",
            "welcome",
            &serde_json::json!({"loginUrl": "https://example.com/login", "tempPassword": "temp123"}),
        )
        .expect("template should render");
    assert!(rendered.contains("https://example.com/login"));
    assert!(rendered.contains("temp123"));
}

#[test]
fn builtin_password_reset_templates_render_in_both_locales() {
    let engine = MailTemplateEngine::with_builtin_locales().expect("engine should initialize");
    let en = engine
        .render(
            "en",
            "password_reset",
            &serde_json::json!({"name": "Niko", "link": "https://example.com/reset"}),
        )
        .expect("english password_reset should render");
    assert!(en.contains("https://example.com/reset"));

    let it = engine
        .render(
            "it",
            "password_reset",
            &serde_json::json!({"name": "Niko", "link": "https://example.com/reset"}),
        )
        .expect("italian password_reset should render");
    assert!(it.contains("https://example.com/reset"));
}

#[test]
fn builtin_email_changed_and_invitation_templates_render() {
    let engine = MailTemplateEngine::with_builtin_locales().expect("engine should initialize");

    let changed = engine
        .render(
            "en",
            "email-changed",
            &serde_json::json!({"newEmail": "new@example.com"}),
        )
        .expect("english email-changed should render");
    assert!(changed.contains("new@example.com"));

    let invitation = engine
        .render(
            "it",
            "invitation",
            &serde_json::json!({"link": "https://example.com/invite"}),
        )
        .expect("italian invitation should render");
    assert!(invitation.contains("https://example.com/invite"));
}

#[test]
fn kebab_case_mail_template_aliases_render() {
    let engine = MailTemplateEngine::with_builtin_locales().expect("engine should initialize");
    let rendered = engine
        .render(
            "en",
            "verify-email",
            &serde_json::json!({"link": "https://example.com/verify"}),
        )
        .expect("kebab-case verify-email should render");
    assert!(rendered.contains("https://example.com/verify"));
}

// ── register may open a session (`issue_session_on_register`) ────────────────
//
// Family spec origin: awesome-go-auth #21. This crate delivers a session as
// the `(AccessToken, RefreshToken)` pair `AuthService::login` returns, so the
// tests assert on that pair, the session rows and the published events.
mod register_session {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;

    use crate::{
        AuthConfig, AuthError, AuthResult, AuthService,
        models::{
            Event, EventType, LinkedOAuthAccount, LoginInput, Session, SignupInput, TenantId, User,
            UserId,
        },
        traits::{EventBus, SessionStore, TelemetryStore, UserStore},
    };

    #[derive(Default)]
    struct MemUsers(Mutex<Vec<User>>);

    #[async_trait]
    impl UserStore for MemUsers {
        async fn create_user(&self, user: User) -> AuthResult<User> {
            self.0.lock().unwrap().push(user.clone());
            Ok(user)
        }
        async fn get_user_by_email(
            &self,
            tenant_id: &TenantId,
            email: &str,
        ) -> AuthResult<Option<User>> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .find(|u| &u.tenant_id == tenant_id && u.email == email)
                .cloned())
        }
        async fn get_user_by_id(&self, user_id: &UserId) -> AuthResult<Option<User>> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .find(|u| &u.id == user_id)
                .cloned())
        }
        async fn update_oauth_links(
            &self,
            _user_id: &UserId,
            _links: Vec<LinkedOAuthAccount>,
        ) -> AuthResult<()> {
            unimplemented!()
        }
        async fn update_profile(
            &self,
            _user_id: &UserId,
            _display_name: Option<&str>,
        ) -> AuthResult<User> {
            unimplemented!()
        }
        async fn set_password_hash(&self, _user_id: &UserId, _hash: &str) -> AuthResult<()> {
            unimplemented!()
        }
        async fn set_email_verified(&self, _user_id: &UserId) -> AuthResult<()> {
            unimplemented!()
        }
        async fn update_email(&self, _user_id: &UserId, _new_email: &str) -> AuthResult<()> {
            unimplemented!()
        }
        async fn set_totp(&self, _user_id: &UserId, _secret: Option<&str>) -> AuthResult<()> {
            unimplemented!()
        }
        async fn delete_user(&self, _user_id: &UserId) -> AuthResult<()> {
            unimplemented!()
        }
    }

    #[derive(Default)]
    struct MemSessions(Mutex<Vec<Session>>);

    impl MemSessions {
        fn rows(&self) -> Vec<Session> {
            self.0.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl SessionStore for MemSessions {
        async fn create_session(&self, session: Session) -> AuthResult<()> {
            self.0.lock().unwrap().push(session);
            Ok(())
        }
        async fn get_session_by_refresh_token(
            &self,
            refresh_token_id: &uuid::Uuid,
        ) -> AuthResult<Option<Session>> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .find(|s| &s.refresh_token_id == refresh_token_id)
                .cloned())
        }
        async fn revoke_session(&self, session_id: &uuid::Uuid) -> AuthResult<()> {
            for s in self.0.lock().unwrap().iter_mut() {
                if &s.id == session_id {
                    s.revoked_at = Some(time::OffsetDateTime::now_utc());
                }
            }
            Ok(())
        }
        async fn list_sessions_for_user(&self, user_id: &UserId) -> AuthResult<Vec<Session>> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .filter(|s| &s.user_id == user_id && s.revoked_at.is_none())
                .cloned()
                .collect())
        }
    }

    /// Records every published event (used as both telemetry store and bus).
    #[derive(Default)]
    struct Recorder(Mutex<Vec<Event>>);

    impl Recorder {
        fn count(&self, pred: fn(&EventType) -> bool) -> usize {
            self.0
                .lock()
                .unwrap()
                .iter()
                .filter(|e| pred(&e.event_type))
                .count()
        }
        fn logins(&self) -> usize {
            self.count(|t| matches!(t, EventType::Login))
        }
        fn signups(&self) -> usize {
            self.count(|t| matches!(t, EventType::Signup))
        }
    }

    #[async_trait]
    impl TelemetryStore for Recorder {
        async fn persist_event(&self, _event: Event) -> AuthResult<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl EventBus for Recorder {
        async fn publish(&self, event: Event) -> AuthResult<()> {
            self.0.lock().unwrap().push(event);
            Ok(())
        }
    }

    struct Harness {
        svc: AuthService<MemUsers, MemSessions, Recorder, Recorder>,
        sessions: Arc<MemSessions>,
        events: Arc<Recorder>,
    }

    fn harness(issue_session_on_register: bool) -> Harness {
        let config = AuthConfig::builder()
            .jwt_secret("test-secret-of-at-least-16-chars")
            .issue_session_on_register(issue_session_on_register)
            .build()
            .expect("config should build");
        let sessions = Arc::new(MemSessions::default());
        let events = Arc::new(Recorder::default());
        let svc = AuthService::new(
            config,
            Arc::new(MemUsers::default()),
            sessions.clone(),
            Arc::new(Recorder::default()),
            events.clone(),
        );
        Harness {
            svc,
            sessions,
            events,
        }
    }

    fn signup_input() -> SignupInput {
        SignupInput {
            email: "new-user@example.com".to_string(),
            password: "correct-horse-battery".to_string(),
            tenant_id: "tenant-test".to_string(),
        }
    }

    #[test]
    fn issue_session_on_register_defaults_to_off() {
        let config = AuthConfig::builder()
            .jwt_secret("test-secret-of-at-least-16-chars")
            .build()
            .expect("config should build");
        assert!(!config.issue_session_on_register);
    }

    #[tokio::test]
    async fn register_off_creates_the_user_and_issues_nothing() {
        let h = harness(false);
        let outcome = h.svc.register(signup_input()).await.expect("register");

        assert_eq!(outcome.user.email, "new-user@example.com");
        assert!(outcome.session.is_none(), "no token pair when off");
        assert!(h.sessions.rows().is_empty(), "no session row when off");
        assert_eq!(h.events.signups(), 1);
        assert_eq!(h.events.logins(), 0, "no login event when off");

        // The client has to log in afterwards, and that still works.
        h.svc
            .login(LoginInput {
                email: "new-user@example.com".to_string(),
                password: "correct-horse-battery".to_string(),
                tenant_id: "tenant-test".to_string(),
            })
            .await
            .expect("login after register");
        assert_eq!(h.sessions.rows().len(), 1);
        assert_eq!(h.events.logins(), 1);
    }

    #[tokio::test]
    async fn register_on_issues_the_same_session_login_issues() {
        let h = harness(true);
        let outcome = h.svc.register(signup_input()).await.expect("register");
        let (access, refresh) = outcome.session.expect("token pair when on");

        assert!(!access.token.is_empty());
        let rows = h.sessions.rows();
        assert_eq!(rows.len(), 1, "exactly one session row, as login creates");
        assert_eq!(rows[0].user_id, outcome.user.id);
        assert_eq!(rows[0].refresh_token_id, refresh.token_id);
        assert!(rows[0].revoked_at.is_none());
        assert_eq!(h.events.signups(), 1);
        assert_eq!(h.events.logins(), 1, "the login event is published");

        // The issued session is live: its refresh token rotates like a login's.
        h.svc
            .rotate_refresh_token(&refresh.token)
            .await
            .expect("refresh token issued on register should rotate");
        assert_eq!(h.svc.list_sessions(&outcome.user.id).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn register_on_refused_registration_issues_nothing() {
        let h = harness(true);
        h.svc.register(signup_input()).await.expect("first register");
        assert_eq!(h.sessions.rows().len(), 1);

        // Duplicate account.
        let dup = h.svc.register(signup_input()).await;
        assert!(matches!(dup, Err(AuthError::Validation(_))));

        // Invalid input.
        let invalid = h
            .svc
            .register(SignupInput {
                email: "not-an-email".to_string(),
                password: "short".to_string(),
                tenant_id: "tenant-test".to_string(),
            })
            .await;
        assert!(matches!(invalid, Err(AuthError::Validation(_))));

        assert_eq!(h.sessions.rows().len(), 1, "refusals create no session row");
        assert_eq!(h.events.logins(), 1, "refusals publish no login event");
        assert_eq!(h.events.signups(), 1);
    }
}
