use indeed_key_webhook::{app, config, db};
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::main]
async fn main() {
    let cfg = config::Config::from_env().expect("config");
    let pool = SqlitePoolOptions::new()
        .connect(&format!("sqlite://{}?mode=rwc", cfg.db_path))
        .await
        .expect("open db");
    db::init(&pool).await.expect("init db");
    let state = app::AppState { pool, secret: cfg.secret };
    let listener = tokio::net::TcpListener::bind(&cfg.bind_addr)
        .await
        .expect("bind");
    println!("listening on {}", cfg.bind_addr);
    axum::serve(listener, app::router(state)).await.expect("serve");
}
