use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::metrics::Metrics;
use crate::{auth, db, validate};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub secret: String,
    pub metrics: Arc<Metrics>,
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
        st.metrics.webhook_unauthorized.fetch_add(1, Ordering::Relaxed);
        tracing::warn!("rejected webhook: bad or missing bearer");
        return StatusCode::UNAUTHORIZED;
    }
    let code = match validate::normalize_code(&body.code) {
        Some(c) => c,
        None => {
            st.metrics.webhook_invalid.fetch_add(1, Ordering::Relaxed);
            tracing::warn!(account = %body.account, "rejected webhook: invalid code");
            return StatusCode::BAD_REQUEST;
        }
    };
    if !validate::valid_account(&body.account) {
        st.metrics.webhook_invalid.fetch_add(1, Ordering::Relaxed);
        tracing::warn!("rejected webhook: empty account");
        return StatusCode::BAD_REQUEST;
    }
    let rec = db::CodeRecord {
        account: body.account.trim().to_string(),
        code,
        timestamp: body.timestamp,
        source: body.source,
    };
    match db::insert(&st.pool, &rec).await {
        Ok(stored) => {
            st.metrics.webhook_ok.fetch_add(1, Ordering::Relaxed);
            if stored {
                st.metrics.codes_stored.fetch_add(1, Ordering::Relaxed);
                tracing::info!(account = %rec.account, "stored code");
            } else {
                tracing::info!(account = %rec.account, "duplicate code ignored");
            }
            StatusCode::OK
        }
        Err(err) => {
            st.metrics.webhook_error.fetch_add(1, Ordering::Relaxed);
            tracing::error!(account = %rec.account, error = %err, "db insert failed");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn get_codes(
    State(st): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<db::CodeRecord>>, StatusCode> {
    if !authed(&headers, &st.secret) {
        st.metrics.codes_unauthorized.fetch_add(1, Ordering::Relaxed);
        tracing::warn!("rejected /codes: bad or missing bearer");
        return Err(StatusCode::UNAUTHORIZED);
    }
    match db::recent(&st.pool, 50).await {
        Ok(rows) => {
            st.metrics.codes_ok.fetch_add(1, Ordering::Relaxed);
            Ok(Json(rows))
        }
        Err(err) => {
            st.metrics.codes_error.fetch_add(1, Ordering::Relaxed);
            tracing::error!(error = %err, "db read failed");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Unauthenticated liveness probe (used by the Docker healthcheck).
async fn health() -> &'static str {
    "ok"
}

/// Unauthenticated Prometheus metrics.
async fn metrics(State(st): State<AppState>) -> String {
    st.metrics.render()
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/webhook", post(post_webhook))
        .route("/codes", get(get_codes))
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .with_state(state)
}
