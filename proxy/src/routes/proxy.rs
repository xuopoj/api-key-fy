use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use http::Request;
use bytes::Bytes;
use chrono::Utc;

use crate::{
    db::{models::App, queries},
    error::AppError,
    proxy::{build_upstream_url, forward},
    AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{slug}/{*path}", any(http_path_handler))
        .route("/{slug}", any(http_path_handler))
}

/// Handler for http_path apps: validates API key, rate limits, proxies, logs.
async fn http_path_handler(
    State(state): State<AppState>,
    Path(params): Path<std::collections::HashMap<String, String>>,
    req: Request<axum::body::Body>,
) -> Result<Response, AppError> {
    let slug = params.get("slug").cloned().unwrap_or_default();
    let sub_path = params.get("path").cloned().unwrap_or_default();

    // Look up app by slug
    let app = queries::get_app_by_slug(&state.db, &slug)
        .await?
        .ok_or(AppError::NotFound)?;

    if app.r#type != "http_path" {
        return Err(AppError::NotFound);
    }

    // Extract and validate API key
    let api_key = extract_bearer(req.headers()).ok_or(AppError::Unauthorized)?;
    let key_record = validate_api_key(&state, &api_key, &app).await?;

    // Rate limit
    if !state.rate_limiter.check(key_record.id, key_record.rate_limit_rpm as u32) {
        return Err(AppError::RateLimitExceeded);
    }

    // Get IAM token using app's credential
    let credential = queries::get_credential_by_id(&state.db, app.credential_id)
        .await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("credential not found")))?;
    let password = crate::crypto::decrypt(&credential.iam_password_enc, &state.config.encryption_key)
        .map_err(|e| AppError::Internal(e))?;
    let iam_token = state.iam.get_token(credential.id, &credential, &password, &state.config.iam_endpoint).await
        .map_err(|e| AppError::Internal(e))?;

    // Build upstream URL
    // Router already stripped the slug; if strip_prefix=false, put it back.
    let req_path = if app.strip_prefix {
        format!("/{sub_path}")
    } else {
        format!("/{slug}/{sub_path}")
    };
    let query = req.uri().query();
    let upstream_url = build_upstream_url(&app, &req_path, query, "")
        .map_err(|e| AppError::Internal(e))?;

    // Collect body
    let method = req.method().clone();
    let headers = req.headers().clone();
    let start = Utc::now();
    let body_bytes = collect_body(req).await?;
    let req_size = body_bytes.len() as i64;

    // Forward
    let response = forward(&state.iam.client(), method.clone(), upstream_url, headers, body_bytes, &iam_token)
        .await
        .map_err(|e| AppError::Internal(e))?;

    let status = response.status().as_u16() as i32;
    let latency_ms = (Utc::now() - start).num_milliseconds() as i32;

    tracing::info!("{} {} {} {}ms", method, req_path, status, latency_ms);

    // Log (fire and forget)
    if key_record.logging_enabled {
        let pool = state.db.clone();
        let key_id = key_record.id;
        let app_id = app.id;
        let method_str = method.to_string();
        let path_str = req_path;
        tokio::spawn(async move {
            let _ = queries::insert_log(
                &pool, Some(key_id), Some(app_id),
                &method_str, &path_str,
                Some(status), Some(latency_ms),
                Some(req_size), None,
            ).await;
        });
    }

    Ok(response)
}

// ── http_port listener ────────────────────────────────────────────────────────

/// Spawn a dedicated TCP listener for each http_port app.
/// Call this at startup for all http_port apps.
pub async fn spawn_port_listeners(state: AppState) -> anyhow::Result<()> {
    let apps = queries::list_apps(&state.db).await?;
    for app in apps {
        if app.r#type == "http_port" {
            if let Some(port) = app.listen_port {
                let state = state.clone();
                tokio::spawn(async move {
                    if let Err(e) = run_port_listener(state, app, port as u16).await {
                        tracing::error!("port listener error on port {port}: {e}");
                    }
                });
            }
        }
    }
    Ok(())
}

async fn run_port_listener(state: AppState, app: App, port: u16) -> anyhow::Result<()> {
    use std::convert::Infallible;
    use tower::service_fn;

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("http_port app '{}' listening on {addr}", app.name);

    // Use tower service_fn to capture state + app without axum's extractor machinery
    let svc = service_fn(move |req: Request<axum::body::Body>| {
        let state = state.clone();
        let app = app.clone();
        async move {
            let resp = handle_port_request(state, app, req)
                .await
                .unwrap_or_else(|e| e.into_response());
            Ok::<_, Infallible>(resp)
        }
    });

    axum::serve(listener, tower::make::Shared::new(svc)).await?;
    Ok(())
}

async fn handle_port_request(
    state: AppState,
    app: App,
    req: Request<axum::body::Body>,
) -> Result<Response, AppError> {
    // Auth: session cookie (browser)
    let user_iam = authenticate_port_request(&state, req.headers()).await?;

    // Check whitelist
    if !app.user_whitelist.is_empty() && !app.user_whitelist.contains(&user_iam) {
        return Err(AppError::Forbidden);
    }

    // Get credential + IAM token
    let credential = queries::get_credential_by_id(&state.db, app.credential_id)
        .await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("credential not found")))?;
    let password = crate::crypto::decrypt(&credential.iam_password_enc, &state.config.encryption_key)
        .map_err(|e| AppError::Internal(e))?;
    let iam_token = state.iam.get_token(credential.id, &credential, &password, &state.config.iam_endpoint).await
        .map_err(|e| AppError::Internal(e))?;

    // Build upstream URL (no slug prefix for port-based apps)
    let req_path = req.uri().path().to_string();
    let query = req.uri().query();
    let upstream_url = build_upstream_url(&app, &req_path, query, "")
        .map_err(|e| AppError::Internal(e))?;

    let method = req.method().clone();
    let headers = req.headers().clone();
    let start = Utc::now();
    let body_bytes = collect_body(req).await?;

    let response = forward(&state.iam.client(), method.clone(), upstream_url, headers, body_bytes, &iam_token)
        .await
        .map_err(|e| AppError::Internal(e))?;

    let latency_ms = (Utc::now() - start).num_milliseconds();
    tracing::info!("{} {} {} {}ms", method, req_path, response.status().as_u16(), latency_ms);

    Ok(response)
}

/// Authenticate a port-proxy request.
/// Returns the IAM username of the authenticated user.
async fn authenticate_port_request(state: &AppState, headers: &axum::http::HeaderMap) -> Result<String, AppError> {
    use axum_extra::extract::cookie::CookieJar;

    // Try session cookie first (browser users)
    let jar = CookieJar::from_headers(headers);
    if let Some(cookie) = jar.get("session") {
        if let Ok(session_id) = cookie.value().parse::<uuid::Uuid>() {
            if let Some(session) = queries::get_session(&state.db, session_id).await? {
                if let Some(user) = queries::get_user_by_id(&state.db, session.user_id).await? {
                    return Ok(user.iam_username);
                }
            }
        }
    }

    Err(AppError::Unauthorized)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn extract_bearer(headers: &axum::http::HeaderMap) -> Option<String> {
    let val = headers.get("Authorization")?.to_str().ok()?;
    val.strip_prefix("Bearer ").map(|s| s.to_string())
}

/// Validate an API key against an app: checks hash, expiry, revocation, scope.
async fn validate_api_key(
    state: &AppState,
    raw_key: &str,
    app: &App,
) -> Result<crate::db::models::ApiKey, AppError> {
    if raw_key.len() < 13 {
        return Err(AppError::Unauthorized);
    }
    let prefix = &raw_key[..13];
    let candidates = queries::get_api_key_by_prefix(&state.db, prefix).await?;

    for key in candidates {
        // Check revocation
        if key.revoked_at.is_some() {
            continue;
        }
        // Check expiry
        if let Some(exp) = key.expires_at {
            if exp < Utc::now() {
                continue;
            }
        }
        // Verify hash
        let matches = bcrypt::verify(raw_key, &key.key_hash)
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        if !matches {
            continue;
        }
        // Check app scope
        let allowed_apps = queries::get_apps_for_key(&state.db, key.id).await?;
        if !allowed_apps.iter().any(|a| a.id == app.id) {
            return Err(AppError::Forbidden);
        }

        return Ok(key);
    }

    Err(AppError::Unauthorized)
}

async fn collect_body(req: Request<axum::body::Body>) -> Result<Bytes, AppError> {
    use http_body_util::BodyExt as _;
    let body = req.into_body();
    let collected = body.collect().await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;
    Ok(collected.to_bytes())
}

