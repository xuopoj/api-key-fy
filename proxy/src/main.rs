mod config;
mod crypto;
mod db;
mod error;
mod iam;
mod proxy;
mod rate_limit;
mod routes;

use anyhow::Result;
use axum::Router;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub config: config::Config,
    pub iam: iam::IamTokenManager,
    pub rate_limiter: rate_limit::RateLimiter,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = config::Config::from_env()?;
    let db = db::connect(&config.database_url).await?;

    // Run migrations on startup
    sqlx::migrate!("./migrations").run(&db).await?;
    tracing::info!("migrations applied");

    let state = AppState {
        db,
        config: config.clone(),
        iam: iam::IamTokenManager::new(),
        rate_limiter: rate_limit::RateLimiter::new(),
    };

    // Spawn dedicated listeners for http_port apps
    routes::proxy::spawn_port_listeners(state.clone()).await?;

    // Spawn log rotation task (runs every hour)
    {
        let pool = state.db.clone();
        let retention_days = state.config.log_retention_days;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600));
            loop {
                interval.tick().await;
                match db::queries::purge_old_logs(&pool, retention_days).await {
                    Ok(n) if n > 0 => tracing::info!("purged {n} old log entries (retention: {retention_days}d)"),
                    Err(e) => tracing::error!("log rotation failed: {e}"),
                    _ => {}
                }
            }
        });
    }

    let app = Router::new()
        .nest("/admin/api", routes::admin::router())
        // Static asset paths must be registered before the proxy wildcard routes
        // so that /assets/*, /favicon.ico, etc. don't get intercepted as proxy slugs.
        .route("/assets/{*path}", axum::routing::get(routes::static_files::handler))
        .route("/favicon.ico", axum::routing::get(routes::static_files::handler))
        .merge(routes::proxy::router())
        // Fallback serves index.html for SPA client-side routing
        .fallback(routes::static_files::handler)
        .with_state(state);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on {addr}");
    axum::serve(listener, app).await?;

    Ok(())
}
