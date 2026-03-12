use std::sync::Arc;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::db::models::Credential;

#[derive(Debug, Clone)]
struct CachedToken {
    token: String,
    expires_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct IamTokenManager {
    cache: Arc<DashMap<Uuid, CachedToken>>, // keyed by credential_id
    client: reqwest::Client,
}

#[derive(Deserialize)]
struct TokenResponse {
    token: TokenBody,
}

#[derive(Deserialize)]
struct TokenBody {
    expires_at: String,
}

impl IamTokenManager {
    pub fn client(&self) -> reqwest::Client {
        self.client.clone()
    }

    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            client: reqwest::Client::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .expect("failed to build reqwest client"),
        }
    }

    /// Get a valid IAM token for the given credential, refreshing if needed.
    /// `password` is the decrypted plaintext password.
    pub async fn get_token(&self, credential_id: Uuid, cred: &Credential, password: &str, iam_endpoint: &str) -> Result<String> {
        if let Some(cached) = self.cache.get(&credential_id) {
            if Self::is_valid(&cached) {
                return Ok(cached.token.clone());
            }
        }

        let (token, expires_at) = self.fetch_token(cred, password, iam_endpoint).await?;

        self.cache.insert(credential_id, CachedToken {
            token: token.clone(),
            expires_at,
        });

        Ok(token)
    }

    async fn fetch_token(&self, cred: &Credential, password: &str, iam_endpoint: &str) -> Result<(String, DateTime<Utc>)> {
        let url = format!("{}/v3/auth/tokens?nocatalog=true", iam_endpoint.trim_end_matches('/'));

        // Use project scope when available, otherwise fall back to domain scope.
        // Domain scope is used for login validation (no project known yet).
        let scope = if !cred.iam_project.is_empty() {
            json!({ "project": { "name": cred.iam_project } })
        } else {
            json!({ "domain": { "name": cred.iam_domain } })
        };

        let body = json!({
            "auth": {
                "identity": {
                    "methods": ["password"],
                    "password": {
                        "user": {
                            "domain": { "name": cred.iam_domain },
                            "name": cred.iam_username,
                            "password": password
                        }
                    }
                },
                "scope": scope
            }
        });

        let resp = self.client
            .post(&url)
            .header("Content-Type", "application/json;charset=utf8")
            .json(&body)
            .send()
            .await
            .context("failed to send IAM token request")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("IAM token request failed ({status}): {text}"));
        }

        let token = resp
            .headers()
            .get("X-Subject-Token")
            .context("X-Subject-Token header missing from IAM response")?
            .to_str()
            .context("X-Subject-Token header is not valid UTF-8")?
            .to_string();

        let body: TokenResponse = resp.json().await.context("failed to parse IAM token response")?;

        let expires_at = DateTime::parse_from_str(&body.token.expires_at, "%Y-%m-%dT%H:%M:%S%.fZ")
            .or_else(|_| DateTime::parse_from_rfc3339(&body.token.expires_at))
            .context("failed to parse IAM token expiry")?
            .with_timezone(&Utc);

        Ok((token, expires_at))
    }

    fn is_valid(cached: &CachedToken) -> bool {
        cached.expires_at - Utc::now() > chrono::Duration::minutes(5)
    }
}
