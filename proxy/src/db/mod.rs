use sqlx::PgPool;

pub mod models;
pub mod queries;

pub async fn connect(database_url: &str) -> sqlx::Result<PgPool> {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
}
