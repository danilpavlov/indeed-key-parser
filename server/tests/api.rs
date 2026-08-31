use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use indeed_key_webhook::{app, db};
use sqlx::SqlitePool;
use tower::ServiceExt;

async fn state() -> app::AppState {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    db::init(&pool).await.unwrap();
    app::AppState {
        pool,
        secret: "s3cr3t".into(),
        metrics: std::sync::Arc::new(indeed_key_webhook::metrics::Metrics::default()),
    }
}

fn post(secret: Option<&str>, json: &str) -> Request<Body> {
    let mut b = Request::builder()
        .method("POST")
        .uri("/webhook")
        .header("content-type", "application/json");
    if let Some(s) = secret {
        b = b.header("authorization", format!("Bearer {s}"));
    }
    b.body(Body::from(json.to_string())).unwrap()
}

const VALID: &str = r#"{"account":"Corp","code":"123 456","timestamp":"2026-08-31T12:00:00Z","source":"com.indeedid.key"}"#;

#[tokio::test]
async fn rejects_without_bearer() {
    let r = app::router(state().await)
        .oneshot(post(None, VALID))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn accepts_and_stores_valid() {
    let st = state().await;
    let r = app::router(st.clone())
        .oneshot(post(Some("s3cr3t"), VALID))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let got = db::recent(&st.pool, 10).await.unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].code, "123456"); // normalized
}

#[tokio::test]
async fn rejects_bad_code() {
    let bad = r#"{"account":"Corp","code":"12","timestamp":"t","source":"s"}"#;
    let r = app::router(state().await)
        .oneshot(post(Some("s3cr3t"), bad))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_codes_requires_auth_and_returns_json() {
    let st = state().await;
    app::router(st.clone())
        .oneshot(post(Some("s3cr3t"), VALID))
        .await
        .unwrap();
    let req = Request::builder()
        .uri("/codes")
        .header("authorization", "Bearer s3cr3t")
        .body(Body::empty())
        .unwrap();
    let r = app::router(st).oneshot(req).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let body = r.into_body().collect().await.unwrap().to_bytes();
    assert!(String::from_utf8_lossy(&body).contains("123456"));
}

#[tokio::test]
async fn health_is_open_and_ok() {
    let req = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();
    let r = app::router(state().await).oneshot(req).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let body = r.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"ok");
}

#[tokio::test]
async fn metrics_is_open_and_counts_a_stored_code() {
    let st = state().await;
    app::router(st.clone())
        .oneshot(post(Some("s3cr3t"), VALID))
        .await
        .unwrap();
    let req = Request::builder()
        .uri("/metrics")
        .body(Body::empty())
        .unwrap();
    let r = app::router(st).oneshot(req).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let body = r.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains("webhook_requests_total{outcome=\"ok\"} 1"), "{text}");
    assert!(text.contains("codes_stored_total 1"), "{text}");
}
