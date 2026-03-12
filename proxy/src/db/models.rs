use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub iam_username: String,
    pub iam_domain: String,
    pub display_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
#[allow(dead_code)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Credential {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub is_shared: bool,
    pub iam_username: String,
    pub iam_password_enc: String,
    pub iam_domain: String,
    pub iam_project: String,
    pub iam_region: String,
    pub iam_endpoint: Option<String>, // custom IAM endpoint, e.g. for private cloud; falls back to https://iam.{region}.myhuaweicloud.com
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct App {
    pub id: Uuid,
    pub name: String,
    pub slug: Option<String>,
    pub r#type: String, // "http_path" | "http_port"
    pub credential_id: Uuid,
    pub base_url: String,
    pub upstream_base_path: String,
    pub strip_prefix: bool,
    pub listen_port: Option<i32>,
    pub user_whitelist: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ApiKey {
    pub id: Uuid,
    pub name: String,
    pub key_hash: String,
    pub key_prefix: String,
    pub key_enc: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub rate_limit_rpm: i32,
    pub logging_enabled: bool,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RequestLog {
    pub id: i64,
    pub api_key_id: Option<Uuid>,
    pub app_id: Option<Uuid>,
    pub method: String,
    pub path: String,
    pub status_code: Option<i32>,
    pub latency_ms: Option<i32>,
    pub request_size_bytes: Option<i64>,
    pub response_size_bytes: Option<i64>,
    pub created_at: DateTime<Utc>,
}
