use shared::State;
use utoipa_axum::{router::OpenApiRouter, routes};

pub const TICKET_TTL_SECONDS: u64 = 60;

pub fn cache_key(token: &str) -> String {
    format!("{}::ticket::{}", crate::settings::PACKAGE, token)
}

mod issue {
    use serde::{Deserialize, Serialize};
    use shared::{
        GetState,
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Deserialize)]
    pub struct Payload {
        pub secret: String,
        pub user_uuid: uuid::Uuid,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        token: String,
        expires_in: u64,
    }

    #[utoipa::path(post, path = "/", responses(
        (status = OK, body = inline(Response)),
    ))]
    pub async fn route(
        state: GetState,
        axum::Json(data): axum::Json<Payload>,
    ) -> ApiResponseResult {
        let settings = state.settings.get().await?;
        let configured = settings
            .get_extension_settings::<crate::settings::SsoSettings>(crate::settings::PACKAGE)
            .ok()
            .and_then(|s| s.shared_secret.clone());

        // A missing secret and a wrong one fail identically, so a caller never learns which.
        if !crate::settings::secret_matches(configured.as_ref(), &data.secret) {
            return ApiResponse::error("invalid credentials")
                .with_status(axum::http::StatusCode::UNAUTHORIZED)
                .ok();
        }

        let token = rand::distr::SampleString::sample_string(
            &rand::distr::Alphanumeric,
            &mut rand::rng(),
            48,
        );

        state
            .cache
            .set(
                &super::cache_key(&token),
                super::TICKET_TTL_SECONDS,
                &data.user_uuid,
            )
            .await?;

        ApiResponse::new_serialized(Response {
            token,
            expires_in: super::TICKET_TTL_SECONDS,
        })
        .ok()
    }
}

mod consume {
    use shared::{
        GetState,
        models::CreatableModel,
        models::user_session::{CreateUserSessionOptions, UserSession},
        response::{ApiResponse, ApiResponseResult},
    };

    #[utoipa::path(get, path = "/{token}", responses(
        (status = TEMPORARY_REDIRECT, body = String),
    ), params(
        ("token" = String, Path, description = "The single-use ticket handed out by the issuing route."),
    ))]
    pub async fn route(
        state: GetState,
        cookies: tower_cookies::Cookies,
        headers: axum::http::HeaderMap,
        axum::extract::Path(token): axum::extract::Path<String>,
        ip: shared::GetIp,
    ) -> ApiResponseResult {
        let key = super::cache_key(&token);

        let Ok(Some(user_uuid)) = state.cache.get::<uuid::Uuid>(&key).await else {
            return ApiResponse::error("this ticket is no longer valid")
                .with_status(axum::http::StatusCode::UNAUTHORIZED)
                .ok();
        };

        // Burned before the session exists, so a replay cannot race a second one through.
        state.cache.invalidate(&key).await?;

        let session = UserSession::create(
            &state,
            CreateUserSessionOptions {
                user_uuid,
                ip: ip.0.into(),
                user_agent: headers
                    .get("User-Agent")
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or("unknown")
                    .into(),
            },
        )
        .await?;

        cookies.add(UserSession::get_cookie(&state, session).await?);

        let settings = state.settings.get().await?;

        ApiResponse::new(axum::body::Body::empty())
            .with_header("Location", settings.app.url.trim_end_matches('/'))
            .with_status(axum::http::StatusCode::TEMPORARY_REDIRECT)
            .ok()
    }
}

mod admin_secret {
    use serde::{Deserialize, Serialize};
    use shared::{
        GetState,
        models::user::GetPermissionManager,
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Deserialize)]
    pub struct Payload {
        pub secret: String,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        configured: bool,
    }

    #[utoipa::path(put, path = "/", responses(
        (status = OK, body = inline(Response)),
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        axum::Json(data): axum::Json<Payload>,
    ) -> ApiResponseResult {
        permissions.has_admin_permission("ssotickets.manage")?;

        if data.secret.len() < 32 {
            return ApiResponse::error("the shared secret must be at least 32 characters")
                .with_status(axum::http::StatusCode::BAD_REQUEST)
                .ok();
        }

        let mut settings = state.settings.get_mut().await?;

        {
            let extension = settings
                .get_mut_extension_settings::<crate::settings::SsoSettings>(crate::settings::PACKAGE)?;
            extension.shared_secret = Some(data.secret.into());
        }

        // save() is what writes to the database; dropping the guard silently discards the change.
        settings.save().await?;

        ApiResponse::new_serialized(Response { configured: true }).ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(issue::route))
        .routes(routes!(consume::route))
        .with_state(state.clone())
}

pub fn admin_router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(admin_secret::route))
        .with_state(state.clone())
}
