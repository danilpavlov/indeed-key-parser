use std::sync::Arc;

use indeed_key_webhook::{app, config, db, metrics::Metrics};
use sqlx::sqlite::SqlitePoolOptions;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let cfg = config::Config::from_env().expect("config");
    let pool = SqlitePoolOptions::new()
        .connect(&format!("sqlite://{}?mode=rwc", cfg.db_path))
        .await
        .expect("open db");
    db::init(&pool).await.expect("init db");
    let state = app::AppState {
        pool,
        secret: cfg.secret,
        metrics: Arc::new(Metrics::default()),
    };
    let listener = tokio::net::TcpListener::bind(&cfg.bind_addr)
        .await
        .expect("bind");
    tracing::info!(addr = %cfg.bind_addr, "listening");
    axum::serve(listener, app::router(state)).await.expect("serve");
}
