use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::{auth, db, validate};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub secret: String,
}

#[derive(Deserialize)]
struct WebhookBody {
    account: String,
    code: String,
    timestamp: String,
    source: String,
}

fn authed(headers: &HeaderMap, secret: &str) -> bool {
    let h = headers.get("authorization").and_then(|v| v.to_str().ok());
    auth::is_authorized(h, secret)
}

async fn post_webhook(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<WebhookBody>,
) -> StatusCode {
    if !authed(&headers, &st.secret) {
        return StatusCode::UNAUTHORIZED;
    }
    let code = match validate::normalize_code(&body.code) {
        Some(c) => c,
        None => return StatusCode::BAD_REQUEST,
    };
    if !validate::valid_account(&body.account) {
        return StatusCode::BAD_REQUEST;
    }
    let rec = db::CodeRecord {
        account: body.account.trim().to_string(),
        code,
        timestamp: body.timestamp,
        source: body.source,
    };
    match db::insert(&st.pool, &rec).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn get_codes(
    State(st): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<db::CodeRecord>>, StatusCode> {
    if !authed(&headers, &st.secret) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    db::recent(&st.pool, 50)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/webhook", post(post_webhook))
        .route("/codes", get(get_codes))
        .with_state(state)
}
