use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use super::models::*;

// ── Users & Sessions ─────────────────────────────────────────────────────────

pub async fn upsert_user(pool: &PgPool, iam_username: &str, iam_domain: &str) -> Result<User> {
    Ok(sqlx::query_as(
        r#"
        INSERT INTO users (iam_username, iam_domain, last_login_at)
        VALUES ($1, $2, NOW())
        ON CONFLICT (iam_username) DO UPDATE SET last_login_at = NOW()
        RETURNING *
        "#,
    )
    .bind(iam_username)
    .bind(iam_domain)
    .fetch_one(pool)
    .await?)
}

pub async fn get_user_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>> {
    Ok(sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?)
}

pub async fn create_session(pool: &PgPool, user_id: Uuid) -> Result<Session> {
    let expires_at = Utc::now() + chrono::Duration::hours(24);
    Ok(sqlx::query_as(
        "INSERT INTO sessions (user_id, expires_at) VALUES ($1, $2) RETURNING *",
    )
    .bind(user_id)
    .bind(expires_at)
    .fetch_one(pool)
    .await?)
}

pub async fn get_session(pool: &PgPool, session_id: Uuid) -> Result<Option<Session>> {
    Ok(
        sqlx::query_as("SELECT * FROM sessions WHERE id = $1 AND expires_at > NOW()")
            .bind(session_id)
            .fetch_optional(pool)
            .await?,
    )
}

pub async fn delete_session(pool: &PgPool, session_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM sessions WHERE id = $1")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Credentials ──────────────────────────────────────────────────────────────

pub async fn list_credentials(pool: &PgPool, user_id: Uuid) -> Result<Vec<Credential>> {
    Ok(sqlx::query_as(
        "SELECT * FROM credentials WHERE owner_id = $1 OR is_shared = true ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?)
}

pub async fn get_credential_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Credential>> {
    Ok(sqlx::query_as("SELECT * FROM credentials WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?)
}

pub async fn create_credential(
    pool: &PgPool,
    owner_id: Uuid,
    name: &str,
    is_shared: bool,
    iam_username: &str,
    iam_password_enc: &str,
    iam_domain: &str,
    iam_project: &str,
    iam_region: &str,
    iam_endpoint: Option<&str>,
) -> Result<Credential> {
    Ok(sqlx::query_as(
        r#"
        INSERT INTO credentials
          (owner_id, name, is_shared, iam_username, iam_password_enc, iam_domain, iam_project, iam_region, iam_endpoint)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#,
    )
    .bind(owner_id).bind(name).bind(is_shared).bind(iam_username).bind(iam_password_enc)
    .bind(iam_domain).bind(iam_project).bind(iam_region).bind(iam_endpoint)
    .fetch_one(pool)
    .await?)
}

pub async fn update_credential(
    pool: &PgPool,
    id: Uuid,
    owner_id: Uuid,
    name: &str,
    is_shared: bool,
    iam_username: &str,
    iam_password_enc: &str,
    iam_domain: &str,
    iam_project: &str,
    iam_region: &str,
    iam_endpoint: Option<&str>,
) -> Result<Option<Credential>> {
    Ok(sqlx::query_as(
        r#"
        UPDATE credentials SET
          name=$3, is_shared=$4, iam_username=$5, iam_password_enc=$6,
          iam_domain=$7, iam_project=$8, iam_region=$9, iam_endpoint=$10, updated_at=NOW()
        WHERE id=$1 AND owner_id=$2
        RETURNING *
        "#,
    )
    .bind(id).bind(owner_id).bind(name).bind(is_shared).bind(iam_username).bind(iam_password_enc)
    .bind(iam_domain).bind(iam_project).bind(iam_region).bind(iam_endpoint)
    .fetch_optional(pool)
    .await?)
}

pub async fn delete_credential(pool: &PgPool, id: Uuid, owner_id: Uuid) -> Result<bool> {
    let result = sqlx::query("DELETE FROM credentials WHERE id=$1 AND owner_id=$2")
        .bind(id)
        .bind(owner_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

// ── Apps ─────────────────────────────────────────────────────────────────────

pub async fn list_apps(pool: &PgPool) -> Result<Vec<App>> {
    Ok(sqlx::query_as("SELECT * FROM apps ORDER BY created_at DESC")
        .fetch_all(pool)
        .await?)
}

pub async fn get_app_by_slug(pool: &PgPool, slug: &str) -> Result<Option<App>> {
    Ok(sqlx::query_as("SELECT * FROM apps WHERE slug=$1")
        .bind(slug)
        .fetch_optional(pool)
        .await?)
}

pub async fn create_app(pool: &PgPool, app: &NewApp) -> Result<App> {
    Ok(sqlx::query_as(
        r#"
        INSERT INTO apps
          (name, slug, type, credential_id, base_url, upstream_base_path, strip_prefix, listen_port, user_whitelist)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#,
    )
    .bind(&app.name).bind(&app.slug).bind(&app.r#type).bind(app.credential_id)
    .bind(&app.base_url).bind(&app.upstream_base_path).bind(app.strip_prefix)
    .bind(app.listen_port).bind(&app.user_whitelist)
    .fetch_one(pool)
    .await?)
}

pub async fn update_app(pool: &PgPool, id: Uuid, app: &NewApp) -> Result<Option<App>> {
    Ok(sqlx::query_as(
        r#"
        UPDATE apps SET
          name=$2, slug=$3, type=$4, credential_id=$5,
          base_url=$6, upstream_base_path=$7, strip_prefix=$8,
          listen_port=$9, user_whitelist=$10, updated_at=NOW()
        WHERE id=$1
        RETURNING *
        "#,
    )
    .bind(id).bind(&app.name).bind(&app.slug).bind(&app.r#type).bind(app.credential_id)
    .bind(&app.base_url).bind(&app.upstream_base_path).bind(app.strip_prefix)
    .bind(app.listen_port).bind(&app.user_whitelist)
    .fetch_optional(pool)
    .await?)
}

pub async fn delete_app(pool: &PgPool, id: Uuid) -> Result<bool> {
    let result = sqlx::query("DELETE FROM apps WHERE id=$1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

// ── API Keys ──────────────────────────────────────────────────────────────────

pub async fn list_api_keys(pool: &PgPool) -> Result<Vec<ApiKey>> {
    Ok(sqlx::query_as("SELECT * FROM api_keys ORDER BY created_at DESC")
        .fetch_all(pool)
        .await?)
}

pub async fn get_api_key_by_prefix(pool: &PgPool, prefix: &str) -> Result<Vec<ApiKey>> {
    Ok(
        sqlx::query_as("SELECT * FROM api_keys WHERE key_prefix=$1 AND revoked_at IS NULL")
            .bind(prefix)
            .fetch_all(pool)
            .await?,
    )
}

pub async fn create_api_key(
    pool: &PgPool,
    name: &str,
    key_hash: &str,
    key_prefix: &str,
    key_enc: &str,
    expires_at: Option<DateTime<Utc>>,
    rate_limit_rpm: i32,
    logging_enabled: bool,
) -> Result<ApiKey> {
    Ok(sqlx::query_as(
        r#"
        INSERT INTO api_keys
          (name, key_hash, key_prefix, key_enc, expires_at, rate_limit_rpm, logging_enabled)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(name).bind(key_hash).bind(key_prefix).bind(key_enc)
    .bind(expires_at).bind(rate_limit_rpm).bind(logging_enabled)
    .fetch_one(pool)
    .await?)
}

pub async fn delete_api_key(pool: &PgPool, id: Uuid) -> Result<bool> {
    let result = sqlx::query("DELETE FROM api_keys WHERE id=$1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn set_api_key_apps(pool: &PgPool, key_id: Uuid, app_ids: &[Uuid]) -> Result<()> {
    sqlx::query("DELETE FROM api_key_apps WHERE api_key_id=$1")
        .bind(key_id)
        .execute(pool)
        .await?;
    for &app_id in app_ids {
        sqlx::query(
            "INSERT INTO api_key_apps (api_key_id, app_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(key_id)
        .bind(app_id)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn get_keys_for_app(pool: &PgPool, app_id: Uuid) -> Result<Vec<ApiKey>> {
    Ok(sqlx::query_as(
        "SELECT k.* FROM api_keys k JOIN api_key_apps ka ON ka.api_key_id = k.id WHERE ka.app_id=$1",
    )
    .bind(app_id)
    .fetch_all(pool)
    .await?)
}

pub async fn get_apps_for_key(pool: &PgPool, key_id: Uuid) -> Result<Vec<App>> {
    Ok(sqlx::query_as(
        "SELECT a.* FROM apps a JOIN api_key_apps ka ON ka.app_id = a.id WHERE ka.api_key_id=$1",
    )
    .bind(key_id)
    .fetch_all(pool)
    .await?)
}

pub struct KeyStats {
    pub api_key_id: Uuid,
    pub total_requests: i64,
    pub last_used_at: Option<DateTime<Utc>>,
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for KeyStats {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Self {
            api_key_id: row.try_get("api_key_id")?,
            total_requests: row.try_get("total_requests")?,
            last_used_at: row.try_get("last_used_at")?,
        })
    }
}

pub async fn get_key_stats(pool: &PgPool) -> Result<Vec<KeyStats>> {
    Ok(sqlx::query_as(
        r#"
        SELECT api_key_id,
               COUNT(*)::bigint as total_requests,
               MAX(created_at) as last_used_at
        FROM request_logs
        WHERE api_key_id IS NOT NULL
        GROUP BY api_key_id
        "#,
    )
    .fetch_all(pool)
    .await?)
}

// ── Logs ─────────────────────────────────────────────────────────────────────

pub async fn purge_old_logs(pool: &PgPool, retention_days: u32) -> Result<u64> {
    let result = sqlx::query(
        "DELETE FROM request_logs WHERE created_at < NOW() - ($1::int * INTERVAL '1 day')",
    )
    .bind(retention_days as i32)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn insert_log(
    pool: &PgPool,
    api_key_id: Option<Uuid>,
    app_id: Option<Uuid>,
    method: &str,
    path: &str,
    status_code: Option<i32>,
    latency_ms: Option<i32>,
    request_size_bytes: Option<i64>,
    response_size_bytes: Option<i64>,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO request_logs
          (api_key_id, app_id, method, path, status_code, latency_ms, request_size_bytes, response_size_bytes)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(api_key_id).bind(app_id).bind(method).bind(path)
    .bind(status_code).bind(latency_ms).bind(request_size_bytes).bind(response_size_bytes)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_logs(
    pool: &PgPool,
    api_key_id: Option<Uuid>,
    app_id: Option<Uuid>,
    limit: i64,
    offset: i64,
) -> Result<Vec<RequestLog>> {
    Ok(sqlx::query_as(
        r#"
        SELECT * FROM request_logs
        WHERE ($1::uuid IS NULL OR api_key_id=$1)
          AND ($2::uuid IS NULL OR app_id=$2)
        ORDER BY created_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(api_key_id).bind(app_id).bind(limit).bind(offset)
    .fetch_all(pool)
    .await?)
}

// ── Helpers ──────────────────────────────────────────────────────────────────

pub struct NewApp {
    pub name: String,
    pub slug: Option<String>,
    pub r#type: String,
    pub credential_id: Uuid,
    pub base_url: String,
    pub upstream_base_path: String,
    pub strip_prefix: bool,
    pub listen_port: Option<i32>,
    pub user_whitelist: Vec<String>,
}
