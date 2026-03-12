use anyhow::{Context, Result};

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub encryption_key: [u8; 32],
    pub iam_endpoint: String,
    pub log_retention_days: u32,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let raw_key = std::env::var("ENCRYPTION_KEY").context("ENCRYPTION_KEY not set")?;
        Ok(Self {
            database_url: std::env::var("DATABASE_URL").context("DATABASE_URL not set")?,
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .context("PORT must be a number")?,
            encryption_key: crate::crypto::parse_key(&raw_key)?,
            iam_endpoint: std::env::var("IAM_ENDPOINT").context("IAM_ENDPOINT not set")?,
            log_retention_days: std::env::var("LOG_RETENTION_DAYS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .context("LOG_RETENTION_DAYS must be a number")?,
        })
    }
}
