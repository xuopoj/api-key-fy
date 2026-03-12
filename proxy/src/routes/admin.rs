use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    crypto::encrypt,
    db::queries,
    error::AppError,
    AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
        // credentials
        .route("/credentials", get(list_credentials).post(create_credential))
        .route("/credentials/{id}", put(update_credential).delete(delete_credential))
        // apps
        .route("/apps", get(list_apps).post(create_app))
        .route("/apps/{id}", put(update_app).delete(delete_app))
        .route("/apps/{id}/keys", get(get_app_keys).put(set_app_keys))
        // api keys
        .route("/keys", get(list_keys).post(create_key))
        .route("/keys/stats", get(key_stats))
        .route("/keys/{id}", delete(delete_key))
        .route("/keys/{id}/reveal", get(reveal_key))
        .route("/keys/{id}/apps", put(set_key_apps))
        // logs
        .route("/logs", get(list_logs))
}

// ── Auth ─────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct LoginRequest {
    iam_username: String,
    iam_password: String,
    iam_domain: String,
}

async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Validate credentials against Huawei IAM by attempting a token fetch
    let dummy_cred = crate::db::models::Credential {
        id: Uuid::nil(),
        name: String::new(),
        owner_id: Uuid::nil(),
        is_shared: false,
        iam_username: body.iam_username.clone(),
        iam_password_enc: String::new(),
        iam_domain: body.iam_domain.clone(),
        iam_project: String::new(), // domain-scoped; project not needed for login validation
        iam_region: String::new(),
        iam_endpoint: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // We use a raw token fetch (not cached) since this is a one-time login validation.
    // Reuse IamTokenManager's fetch logic via a temporary cred.
    state.iam
        .get_token(Uuid::nil(), &dummy_cred, &body.iam_password, &state.config.iam_endpoint)
        .await
        .map_err(|e| { tracing::warn!("login IAM validation failed: {e}"); AppError::Unauthorized })?;

    let user = queries::upsert_user(&state.db, &body.iam_username, &body.iam_domain).await?;
    let session = queries::create_session(&state.db, user.id).await?;

    let cookie = Cookie::build(("session", session.id.to_string()))
        .http_only(true)
        .path("/")
        .build();

    Ok((jar.add(cookie), Json(UserResponse::from(user))))
}

async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    if let Some(cookie) = jar.get("session") {
        if let Ok(session_id) = cookie.value().parse::<Uuid>() {
            queries::delete_session(&state.db, session_id).await?;
        }
    }
    let removed = Cookie::build(("session", "")).path("/").build();
    Ok((jar.remove(removed), StatusCode::NO_CONTENT))
}

async fn me(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    let user = require_session(&state, &jar).await?;
    Ok(Json(UserResponse::from(user)))
}

// ── Credentials ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CredentialRequest {
    name: String,
    is_shared: bool,
    iam_username: String,
    iam_password: String, // plaintext; encrypted before storing
    iam_domain: String,
    iam_project: String,
    iam_region: String,
    iam_endpoint: Option<String>,
}

#[derive(Serialize)]
struct CredentialResponse {
    id: Uuid,
    name: String,
    is_shared: bool,
    iam_username: String,
    iam_domain: String,
    iam_project: String,
    iam_region: String,
    iam_endpoint: Option<String>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

impl From<crate::db::models::Credential> for CredentialResponse {
    fn from(c: crate::db::models::Credential) -> Self {
        Self {
            id: c.id,
            name: c.name,
            is_shared: c.is_shared,
            iam_username: c.iam_username,
            iam_domain: c.iam_domain,
            iam_project: c.iam_project,
            iam_region: c.iam_region,
            iam_endpoint: c.iam_endpoint,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

async fn list_credentials(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    let user = require_session(&state, &jar).await?;
    let creds = queries::list_credentials(&state.db, user.id).await?;
    Ok(Json(creds.into_iter().map(CredentialResponse::from).collect::<Vec<_>>()))
}

async fn create_credential(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<CredentialRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = require_session(&state, &jar).await?;
    let enc = encrypt(&body.iam_password, &state.config.encryption_key)
        .map_err(|e| AppError::Internal(e))?;
    let cred = queries::create_credential(
        &state.db, user.id, &body.name, body.is_shared,
        &body.iam_username, &enc, &body.iam_domain,
        &body.iam_project, &body.iam_region, body.iam_endpoint.as_deref(),
    ).await?;
    Ok((StatusCode::CREATED, Json(CredentialResponse::from(cred))))
}

async fn update_credential(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<Uuid>,
    Json(body): Json<CredentialRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = require_session(&state, &jar).await?;
    let enc = encrypt(&body.iam_password, &state.config.encryption_key)
        .map_err(|e| AppError::Internal(e))?;
    let cred = queries::update_credential(
        &state.db, id, user.id, &body.name, body.is_shared,
        &body.iam_username, &enc, &body.iam_domain,
        &body.iam_project, &body.iam_region, body.iam_endpoint.as_deref(),
    ).await?.ok_or(AppError::NotFound)?;
    Ok(Json(CredentialResponse::from(cred)))
}

async fn delete_credential(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = require_session(&state, &jar).await?;
    let deleted = queries::delete_credential(&state.db, id, user.id).await?;
    if deleted { Ok(StatusCode::NO_CONTENT) } else { Err(AppError::NotFound) }
}

// ── Apps ─────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct AppRequest {
    name: String,
    slug: Option<String>,
    r#type: String,
    credential_id: Uuid,
    base_url: String,
    upstream_base_path: Option<String>,
    strip_prefix: Option<bool>,
    listen_port: Option<i32>,
    user_whitelist: Option<Vec<String>>,
}

async fn list_apps(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    require_session(&state, &jar).await?;
    let apps = queries::list_apps(&state.db).await?;
    Ok(Json(apps))
}

async fn create_app(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<AppRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_session(&state, &jar).await?;
    validate_app_type(&body)?;
    let app = queries::create_app(&state.db, &body.into()).await?;
    Ok((StatusCode::CREATED, Json(app)))
}

async fn update_app(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<Uuid>,
    Json(body): Json<AppRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_session(&state, &jar).await?;
    validate_app_type(&body)?;
    let app = queries::update_app(&state.db, id, &body.into()).await?.ok_or(AppError::NotFound)?;
    Ok(Json(app))
}

async fn get_app_keys(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_session(&state, &jar).await?;
    let keys = queries::get_keys_for_app(&state.db, id).await?;
    Ok(Json(keys.into_iter().map(ApiKeyMeta::from).collect::<Vec<_>>()))
}

async fn set_app_keys(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(app_id): Path<Uuid>,
    Json(key_ids): Json<Vec<Uuid>>,
) -> Result<impl IntoResponse, AppError> {
    require_session(&state, &jar).await?;
    // For each key, add this app to its allowed apps (preserving existing apps)
    for key_id in &key_ids {
        let mut existing: Vec<Uuid> = queries::get_apps_for_key(&state.db, *key_id)
            .await?
            .into_iter()
            .map(|a| a.id)
            .collect();
        if !existing.contains(&app_id) {
            existing.push(app_id);
            queries::set_api_key_apps(&state.db, *key_id, &existing).await?;
        }
    }
    // Remove this app from keys that are no longer in the list
    let all_keys = queries::get_keys_for_app(&state.db, app_id).await?;
    for key in all_keys {
        if !key_ids.contains(&key.id) {
            let remaining: Vec<Uuid> = queries::get_apps_for_key(&state.db, key.id)
                .await?
                .into_iter()
                .map(|a| a.id)
                .filter(|&id| id != app_id)
                .collect();
            queries::set_api_key_apps(&state.db, key.id, &remaining).await?;
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_app(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_session(&state, &jar).await?;
    let deleted = queries::delete_app(&state.db, id).await?;
    if deleted { Ok(StatusCode::NO_CONTENT) } else { Err(AppError::NotFound) }
}

fn validate_app_type(body: &AppRequest) -> Result<(), AppError> {
    match body.r#type.as_str() {
        "http_path" => {
            if body.slug.is_none() {
                return Err(AppError::BadRequest("slug required for http_path apps".into()));
            }
        }
        "http_port" => {
            if body.listen_port.is_none() {
                return Err(AppError::BadRequest("listen_port required for http_port apps".into()));
            }
        }
        _ => return Err(AppError::BadRequest("type must be http_path or http_port".into())),
    }
    Ok(())
}

impl From<AppRequest> for queries::NewApp {
    fn from(b: AppRequest) -> Self {
        Self {
            name: b.name,
            slug: b.slug,
            r#type: b.r#type,
            credential_id: b.credential_id,
            base_url: b.base_url,
            upstream_base_path: b.upstream_base_path.unwrap_or_else(|| "/".into()),
            strip_prefix: b.strip_prefix.unwrap_or(true),
            listen_port: b.listen_port,
            user_whitelist: b.user_whitelist.unwrap_or_default(),
        }
    }
}

// ── API Keys ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CreateKeyRequest {
    name: String,
    expires_at: Option<chrono::DateTime<Utc>>,
    rate_limit_rpm: Option<i32>,
    logging_enabled: Option<bool>,
    app_ids: Vec<Uuid>,
}

#[derive(Serialize)]
struct CreateKeyResponse {
    key: String, // only returned once
    #[serde(flatten)]
    meta: ApiKeyMeta,
}

#[derive(Serialize)]
struct ApiKeyMeta {
    id: Uuid,
    name: String,
    key_prefix: String,
    expires_at: Option<chrono::DateTime<Utc>>,
    rate_limit_rpm: i32,
    logging_enabled: bool,
    revoked_at: Option<chrono::DateTime<Utc>>,
    created_at: chrono::DateTime<Utc>,
}

impl From<crate::db::models::ApiKey> for ApiKeyMeta {
    fn from(k: crate::db::models::ApiKey) -> Self {
        Self {
            id: k.id,
            name: k.name,
            key_prefix: k.key_prefix,
            expires_at: k.expires_at,
            rate_limit_rpm: k.rate_limit_rpm,
            logging_enabled: k.logging_enabled,
            revoked_at: k.revoked_at,
            created_at: k.created_at,
        }
    }
}

async fn key_stats(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    require_session(&state, &jar).await?;
    let stats = queries::get_key_stats(&state.db).await?;
    let json: Vec<_> = stats.iter().map(|s| serde_json::json!({
        "api_key_id": s.api_key_id,
        "total_requests": s.total_requests,
        "last_used_at": s.last_used_at,
    })).collect();
    Ok(Json(json))
}

async fn list_keys(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    require_session(&state, &jar).await?;
    let keys = queries::list_api_keys(&state.db).await?;
    Ok(Json(keys.into_iter().map(ApiKeyMeta::from).collect::<Vec<_>>()))
}

async fn create_key(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<CreateKeyRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_session(&state, &jar).await?;

    // Generate key: "akfy_" + 32 random hex chars
    let raw_key = generate_api_key();
    let prefix = raw_key[..13].to_string(); // "akfy_" + 8 chars
    let hash = bcrypt::hash(&raw_key, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    let key_enc = crate::crypto::encrypt(&raw_key, &state.config.encryption_key)
        .map_err(|e| AppError::Internal(e))?;

    let key = queries::create_api_key(
        &state.db,
        &body.name,
        &hash,
        &prefix,
        &key_enc,
        body.expires_at,
        body.rate_limit_rpm.unwrap_or(60),
        body.logging_enabled.unwrap_or(true),
    ).await?;

    queries::set_api_key_apps(&state.db, key.id, &body.app_ids).await?;

    Ok((StatusCode::CREATED, Json(CreateKeyResponse {
        key: raw_key,
        meta: ApiKeyMeta::from(key),
    })))
}

async fn reveal_key(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_session(&state, &jar).await?;
    let key = queries::list_api_keys(&state.db).await?
        .into_iter()
        .find(|k| k.id == id)
        .ok_or(AppError::NotFound)?;
    let enc = key.key_enc.ok_or(AppError::NotFound)?;
    let raw = crate::crypto::decrypt(&enc, &state.config.encryption_key)
        .map_err(|e| AppError::Internal(e))?;
    Ok(Json(serde_json::json!({ "key": raw })))
}

async fn delete_key(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_session(&state, &jar).await?;
    let deleted = queries::delete_api_key(&state.db, id).await?;
    if deleted { Ok(StatusCode::NO_CONTENT) } else { Err(AppError::NotFound) }
}

async fn set_key_apps(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<Uuid>,
    Json(app_ids): Json<Vec<Uuid>>,
) -> Result<impl IntoResponse, AppError> {
    require_session(&state, &jar).await?;
    queries::set_api_key_apps(&state.db, id, &app_ids).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Logs ─────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct LogsQuery {
    api_key_id: Option<Uuid>,
    app_id: Option<Uuid>,
    limit: Option<i64>,
    offset: Option<i64>,
}

async fn list_logs(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(q): Query<LogsQuery>,
) -> Result<impl IntoResponse, AppError> {
    require_session(&state, &jar).await?;
    let logs = queries::list_logs(
        &state.db,
        q.api_key_id,
        q.app_id,
        q.limit.unwrap_or(50).min(200),
        q.offset.unwrap_or(0),
    ).await?;
    Ok(Json(logs))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct UserResponse {
    id: Uuid,
    iam_username: String,
    iam_domain: String,
    display_name: Option<String>,
}

impl From<crate::db::models::User> for UserResponse {
    fn from(u: crate::db::models::User) -> Self {
        Self { id: u.id, iam_username: u.iam_username, iam_domain: u.iam_domain, display_name: u.display_name }
    }
}

async fn require_session(
    state: &AppState,
    jar: &CookieJar,
) -> Result<crate::db::models::User, AppError> {
    let session_id = jar
        .get("session")
        .and_then(|c| c.value().parse::<Uuid>().ok())
        .ok_or(AppError::Unauthorized)?;

    let session = queries::get_session(&state.db, session_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    queries::get_user_by_id(&state.db, session.user_id)
        .await?
        .ok_or(AppError::Unauthorized)
}

fn generate_api_key() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 20];
    rand::rng().fill_bytes(&mut bytes);
    format!("akfy_{}", hex::encode(bytes))
}
