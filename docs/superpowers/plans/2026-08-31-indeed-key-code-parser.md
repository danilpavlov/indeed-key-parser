# Indeed Key Code Parser Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Read the user's own OTP codes (with their account label) from the Indeed Key Android app screen and forward each to a self-hosted Rust webhook that stores them.

**Architecture:** Two components in a monorepo. `server/` is a Rust `axum` webhook that authenticates via a shared bearer secret and stores codes in SQLite. `android/` is a Kotlin app whose `AccessibilityService` reads the code + account label from the Indeed Key UI tree and POSTs them to the webhook, with dedup and offline retry. The pairing/extraction logic and the server storage/auth logic are pure functions so they are unit-testable; framework glue is verified manually.

**Tech Stack:** Rust (axum 0.7, tokio, sqlx 0.8 with sqlite, serde), Kotlin (Android SDK, OkHttp, WorkManager, EncryptedSharedPreferences), JUnit.

**Spec:** `docs/superpowers/specs/2026-08-31-indeed-key-code-parser-design.md`

## Global Constraints

- Payload contract (verbatim), sent as JSON POST body and stored:
  `{ "account": "<name>", "code": "<6-8 digits>", "timestamp": "<RFC3339>", "source": "com.indeedid.key" }`
- Auth on every server endpoint: header `Authorization: Bearer <secret>`.
- Code validation everywhere: 6–8 ASCII digits, spaces stripped before validation.
- Server config from env only: `WEBHOOK_SECRET` (required), `BIND_ADDR` (default `0.0.0.0:8080`), `DB_PATH` (default `codes.db`).
- Dedup key: `(account, code)` on the app; DB uniqueness `(account, code, timestamp)` on the server.
- Rust edition 2021. Kotlin app targets minSdk 26 (needed for `EncryptedSharedPreferences`).
- Commits: this repo runs under an auto-mode classifier that blocks `git commit` for this session's topic. Commit steps are written for execution outside auto-mode (default permission mode) — run them there.

---

## Task 0: Spike — confirm Indeed Key exposes code + label via accessibility

**Not a code task.** Deliverable: `docs/superpowers/notes/2026-08-31-spike-accessibility.md` recording findings. All later Android tasks depend on the answer.

- [ ] **Step 1: Build a throwaway accessibility dumper.** In a scratch app (or reuse Task 6's service temporarily), on any window event for package `com.indeedid.key` log the full node tree: for each node print `className`, `text`, `viewIdResourceName`, and depth. Enable it in system settings, open Indeed Key.
- [ ] **Step 2: Record findings** in the notes file, answering:
  1. Is the numeric **code** present as a node `text` (not blank / not `FLAG_SECURE`-blocked)?
  2. Is the **account label** present as a nearby node? What is the tree relationship (same parent container / adjacent row)? Copy 1–2 real sample subtrees (redact the actual code digits).
- [ ] **Step 3: Decide.** If the code is exposed → proceed with this plan (path 1). If it is blank/blocked → STOP and revisit spec "path 2" (MediaProjection + OCR); do not continue the Android tasks. Note the decision explicitly.
- [ ] **Step 4: Commit** the notes file.

```bash
git add docs/superpowers/notes/2026-08-31-spike-accessibility.md
git commit -m "docs: record accessibility spike findings for Indeed Key"
```

---

## Task 1: Rust server scaffold + config

**Files:**
- Create: `server/Cargo.toml`
- Create: `server/src/config.rs`
- Create: `server/src/main.rs` (minimal stub for now)
- Test: unit tests inside `server/src/config.rs`

**Interfaces:**
- Produces: `Config { secret: String, bind_addr: String, db_path: String }`; `Config::from_getter(f: impl Fn(&str) -> Option<String>) -> Result<Config, String>`; `Config::from_env() -> Result<Config, String>`.

- [ ] **Step 1: Write `Cargo.toml`**

```toml
[package]
name = "indeed-key-webhook"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }

[dev-dependencies]
tower = { version = "0.5", features = ["util"] }
http-body-util = "0.1"
```

- [ ] **Step 2: Write the failing test** in `server/src/config.rs`

```rust
pub struct Config {
    pub secret: String,
    pub bind_addr: String,
    pub db_path: String,
}

impl Config {
    pub fn from_getter(get: impl Fn(&str) -> Option<String>) -> Result<Config, String> {
        let secret = get("WEBHOOK_SECRET").ok_or("WEBHOOK_SECRET not set")?;
        if secret.is_empty() {
            return Err("WEBHOOK_SECRET is empty".into());
        }
        Ok(Config {
            secret,
            bind_addr: get("BIND_ADDR").unwrap_or_else(|| "0.0.0.0:8080".into()),
            db_path: get("DB_PATH").unwrap_or_else(|| "codes.db".into()),
        })
    }

    pub fn from_env() -> Result<Config, String> {
        Self::from_getter(|k| std::env::var(k).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn getter(map: HashMap<&'static str, &'static str>) -> impl Fn(&str) -> Option<String> {
        move |k| map.get(k).map(|s| s.to_string())
    }

    #[test]
    fn defaults_apply_when_only_secret_set() {
        let cfg = Config::from_getter(getter(HashMap::from([("WEBHOOK_SECRET", "s3cr3t")]))).unwrap();
        assert_eq!(cfg.secret, "s3cr3t");
        assert_eq!(cfg.bind_addr, "0.0.0.0:8080");
        assert_eq!(cfg.db_path, "codes.db");
    }

    #[test]
    fn missing_secret_is_error() {
        let err = Config::from_getter(getter(HashMap::new())).unwrap_err();
        assert!(err.contains("WEBHOOK_SECRET"));
    }
}
```

- [ ] **Step 3: Write minimal `main.rs`** so the crate compiles

```rust
mod config;

#[tokio::main]
async fn main() {
    let cfg = config::Config::from_env().expect("config");
    println!("would bind {}", cfg.bind_addr);
}
```

- [ ] **Step 4: Run tests**

Run: `cd server && cargo test config::`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add server/Cargo.toml server/src/config.rs server/src/main.rs
git commit -m "feat(server): scaffold rust webhook + env config"
```

---

## Task 2: Storage layer (SQLite)

**Files:**
- Create: `server/src/db.rs`
- Modify: `server/src/main.rs` (add `mod db;`)
- Test: unit tests inside `server/src/db.rs` (use `sqlite::memory:`)

**Interfaces:**
- Produces:
  - `struct CodeRecord { account: String, code: String, timestamp: String, source: String }` (derives `serde::Serialize`, `serde::Deserialize`, `sqlx::FromRow`, `Debug`, `Clone`, `PartialEq`).
  - `async fn init(pool: &SqlitePool) -> sqlx::Result<()>`
  - `async fn insert(pool: &SqlitePool, rec: &CodeRecord) -> sqlx::Result<bool>` (returns `true` if a new row was inserted, `false` if it was a duplicate).
  - `async fn recent(pool: &SqlitePool, limit: i64) -> sqlx::Result<Vec<CodeRecord>>` (newest first).

- [ ] **Step 1: Write `db.rs` with the failing tests**

```rust
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct CodeRecord {
    pub account: String,
    pub code: String,
    pub timestamp: String,
    pub source: String,
}

pub async fn init(pool: &SqlitePool) -> sqlx::Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS codes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            account TEXT NOT NULL,
            code TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            source TEXT NOT NULL,
            UNIQUE(account, code, timestamp)
        )",
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert(pool: &SqlitePool, rec: &CodeRecord) -> sqlx::Result<bool> {
    let res = sqlx::query(
        "INSERT OR IGNORE INTO codes (account, code, timestamp, source) VALUES (?, ?, ?, ?)",
    )
    .bind(&rec.account)
    .bind(&rec.code)
    .bind(&rec.timestamp)
    .bind(&rec.source)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn recent(pool: &SqlitePool, limit: i64) -> sqlx::Result<Vec<CodeRecord>> {
    sqlx::query_as::<_, CodeRecord>(
        "SELECT account, code, timestamp, source FROM codes ORDER BY id DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init(&pool).await.unwrap();
        pool
    }

    fn rec() -> CodeRecord {
        CodeRecord {
            account: "Corp".into(),
            code: "123456".into(),
            timestamp: "2026-08-31T12:00:00Z".into(),
            source: "com.indeedid.key".into(),
        }
    }

    #[tokio::test]
    async fn insert_then_recent_returns_it() {
        let pool = pool().await;
        assert!(insert(&pool, &rec()).await.unwrap());
        let got = recent(&pool, 10).await.unwrap();
        assert_eq!(got, vec![rec()]);
    }

    #[tokio::test]
    async fn duplicate_is_ignored() {
        let pool = pool().await;
        assert!(insert(&pool, &rec()).await.unwrap());
        assert!(!insert(&pool, &rec()).await.unwrap());
        assert_eq!(recent(&pool, 10).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn recent_is_newest_first() {
        let pool = pool().await;
        let mut a = rec();
        a.timestamp = "2026-08-31T12:00:00Z".into();
        let mut b = rec();
        b.timestamp = "2026-08-31T12:00:30Z".into();
        insert(&pool, &a).await.unwrap();
        insert(&pool, &b).await.unwrap();
        let got = recent(&pool, 10).await.unwrap();
        assert_eq!(got[0].timestamp, "2026-08-31T12:00:30Z");
    }
}
```

- [ ] **Step 2: Add `mod db;` to `main.rs`** (below `mod config;`).

- [ ] **Step 3: Run tests**

Run: `cd server && cargo test db::`
Expected: PASS (3 tests).

- [ ] **Step 4: Commit**

```bash
git add server/src/db.rs server/src/main.rs
git commit -m "feat(server): sqlite storage with idempotent insert"
```

---

## Task 3: Auth + validation helpers

**Files:**
- Create: `server/src/auth.rs`
- Create: `server/src/validate.rs`
- Modify: `server/src/main.rs` (add `mod auth;` and `mod validate;`)
- Test: unit tests inside both files

**Interfaces:**
- Produces:
  - `fn is_authorized(header: Option<&str>, secret: &str) -> bool` (true only for exactly `Bearer <secret>`).
  - `fn normalize_code(raw: &str) -> Option<String>` (strips spaces; returns the digits if 6–8 ASCII digits, else `None`).
  - `fn valid_account(account: &str) -> bool` (non-empty after trim).

- [ ] **Step 1: Write `auth.rs`**

```rust
pub fn is_authorized(header: Option<&str>, secret: &str) -> bool {
    match header.and_then(|v| v.strip_prefix("Bearer ")) {
        Some(token) => !secret.is_empty() && token == secret,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_matching_bearer() {
        assert!(is_authorized(Some("Bearer abc"), "abc"));
    }

    #[test]
    fn rejects_wrong_or_missing() {
        assert!(!is_authorized(Some("Bearer nope"), "abc"));
        assert!(!is_authorized(Some("abc"), "abc"));
        assert!(!is_authorized(None, "abc"));
        assert!(!is_authorized(Some("Bearer "), ""));
    }
}
```

- [ ] **Step 2: Write `validate.rs`**

```rust
pub fn normalize_code(raw: &str) -> Option<String> {
    let digits: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    if (6..=8).contains(&digits.len()) && digits.chars().all(|c| c.is_ascii_digit()) {
        Some(digits)
    } else {
        None
    }
}

pub fn valid_account(account: &str) -> bool {
    !account.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_spaced_code() {
        assert_eq!(normalize_code("123 456").as_deref(), Some("123456"));
    }

    #[test]
    fn rejects_bad_codes() {
        assert!(normalize_code("12345").is_none()); // too short
        assert!(normalize_code("123456789").is_none()); // too long
        assert!(normalize_code("12a456").is_none()); // non-digit
    }

    #[test]
    fn account_must_be_non_empty() {
        assert!(valid_account("Corp"));
        assert!(!valid_account("   "));
    }
}
```

- [ ] **Step 3: Add `mod auth;` and `mod validate;` to `main.rs`.**

- [ ] **Step 4: Run tests**

Run: `cd server && cargo test auth:: validate::`
Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
git add server/src/auth.rs server/src/validate.rs server/src/main.rs
git commit -m "feat(server): bearer auth + payload validation helpers"
```

---

## Task 4: HTTP handlers + router + integration tests

**Files:**
- Create: `server/src/app.rs` (router factory + handlers)
- Modify: `server/src/main.rs` (build the app and serve)
- Test: `server/tests/api.rs` (integration via `tower::ServiceExt::oneshot`)

**Interfaces:**
- Consumes: `db::{CodeRecord, init, insert, recent}`, `auth::is_authorized`, `validate::{normalize_code, valid_account}`, `config::Config`.
- Produces:
  - `struct AppState { pool: SqlitePool, secret: String }` (derive `Clone`).
  - `fn router(state: AppState) -> axum::Router`.
  - Route `POST /webhook`: 401 if unauthorized; 400 if `account` empty or `code` not 6–8 digits; else insert (normalized code) and return 200 (also 200 on duplicate).
  - Route `GET /codes`: 401 if unauthorized; else 200 with JSON array of the last 50 `CodeRecord`, newest first.

- [ ] **Step 1: Write `app.rs`**

```rust
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
```

- [ ] **Step 2: Wire `main.rs`**

```rust
mod app;
mod auth;
mod config;
mod db;
mod validate;

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
    let listener = tokio::net::TcpListener::bind(&cfg.bind_addr).await.expect("bind");
    println!("listening on {}", cfg.bind_addr);
    axum::serve(listener, app::router(state)).await.expect("serve");
}
```

- [ ] **Step 3: Write `tests/api.rs`**

```rust
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use indeed_key_webhook::{app, db};
use sqlx::SqlitePool;
use tower::ServiceExt;

async fn state() -> app::AppState {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    db::init(&pool).await.unwrap();
    app::AppState { pool, secret: "s3cr3t".into() }
}

fn post(secret: Option<&str>, json: &str) -> Request<Body> {
    let mut b = Request::builder().method("POST").uri("/webhook")
        .header("content-type", "application/json");
    if let Some(s) = secret {
        b = b.header("authorization", format!("Bearer {s}"));
    }
    b.body(Body::from(json.to_string())).unwrap()
}

const VALID: &str = r#"{"account":"Corp","code":"123 456","timestamp":"2026-08-31T12:00:00Z","source":"com.indeedid.key"}"#;

#[tokio::test]
async fn rejects_without_bearer() {
    let r = app::router(state().await).oneshot(post(None, VALID)).await.unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn accepts_and_stores_valid() {
    let st = state().await;
    let r = app::router(st.clone()).oneshot(post(Some("s3cr3t"), VALID)).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let got = db::recent(&st.pool, 10).await.unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].code, "123456"); // normalized
}

#[tokio::test]
async fn rejects_bad_code() {
    let bad = r#"{"account":"Corp","code":"12","timestamp":"t","source":"s"}"#;
    let r = app::router(state().await).oneshot(post(Some("s3cr3t"), bad)).await.unwrap();
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_codes_requires_auth_and_returns_json() {
    let st = state().await;
    app::router(st.clone()).oneshot(post(Some("s3cr3t"), VALID)).await.unwrap();
    let req = Request::builder().uri("/codes")
        .header("authorization", "Bearer s3cr3t").body(Body::empty()).unwrap();
    let r = app::router(st).oneshot(req).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let body = r.into_body().collect().await.unwrap().to_bytes();
    assert!(String::from_utf8_lossy(&body).contains("123456"));
}
```

- [ ] **Step 4: Expose modules to the integration test.** Create `server/src/lib.rs` re-exporting the modules and make `main.rs` use the crate. Add to `Cargo.toml`:

```toml
[lib]
name = "indeed_key_webhook"
path = "src/lib.rs"

[[bin]]
name = "indeed-key-webhook"
path = "src/main.rs"
```

`server/src/lib.rs`:

```rust
pub mod app;
pub mod auth;
pub mod config;
pub mod db;
pub mod validate;
```

Then reduce `main.rs` to `use indeed_key_webhook::{app, config, db};` plus the `#[tokio::main]` body from Step 2 (drop the `mod` lines).

- [ ] **Step 5: Run tests**

Run: `cd server && cargo test`
Expected: PASS (all unit + 4 integration tests).

- [ ] **Step 6: Commit**

```bash
git add server/
git commit -m "feat(server): webhook + codes endpoints with integration tests"
```

---

## Task 5: Android `extractEntry` pure function

**Files:**
- Create: `android/app/src/main/java/com/indeedkeyparser/parse/UiNode.kt`
- Create: `android/app/src/main/java/com/indeedkeyparser/parse/CodeExtractor.kt`
- Test: `android/app/src/test/java/com/indeedkeyparser/parse/CodeExtractorTest.kt`

**Interfaces:**
- Produces:
  - `data class UiNode(val text: String?, val children: List<UiNode> = emptyList())`
  - `data class Entry(val account: String, val code: String)`
  - `fun extractEntry(root: UiNode): Entry?` — finds a code node (6–8 digits, spaces stripped) and pairs it with the first sibling label text in the same container; returns `null` if none.

- [ ] **Step 1: Write `UiNode.kt`**

```kotlin
package com.indeedkeyparser.parse

data class UiNode(val text: String?, val children: List<UiNode> = emptyList())

data class Entry(val account: String, val code: String)
```

- [ ] **Step 2: Write the failing test `CodeExtractorTest.kt`**

```kotlin
package com.indeedkeyparser.parse

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CodeExtractorTest {
    @Test
    fun pairs_code_with_label_in_same_container() {
        val root = UiNode(null, listOf(
            UiNode(null, listOf(UiNode("My Corp"), UiNode("123456")))
        ))
        assertEquals(Entry("My Corp", "123456"), extractEntry(root))
    }

    @Test
    fun strips_spaces_from_code() {
        val root = UiNode(null, listOf(
            UiNode(null, listOf(UiNode("Bank"), UiNode("123 456")))
        ))
        assertEquals(Entry("Bank", "123456"), extractEntry(root))
    }

    @Test
    fun returns_null_when_no_code() {
        val root = UiNode(null, listOf(UiNode(null, listOf(UiNode("Just text")))))
        assertNull(extractEntry(root))
    }
}
```

- [ ] **Step 3: Write `CodeExtractor.kt`**

```kotlin
package com.indeedkeyparser.parse

private val CODE_REGEX = Regex("^\\d{6,8}$")

private fun isCode(raw: String): Boolean = CODE_REGEX.matches(raw.replace(" ", ""))

fun extractEntry(root: UiNode): Entry? {
    val texts = root.children.mapNotNull { it.text?.trim() }.filter { it.isNotEmpty() }
    val codeText = texts.firstOrNull { isCode(it) }
    if (codeText != null) {
        val label = texts.firstOrNull { !isCode(it) }
        if (label != null) return Entry(label, codeText.replace(" ", ""))
    }
    for (child in root.children) {
        extractEntry(child)?.let { return it }
    }
    return null
}
```

- [ ] **Step 4: Run tests**

Run: `cd android && ./gradlew :app:testDebugUnitTest --tests "com.indeedkeyparser.parse.CodeExtractorTest"`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add android/app/src/main/java/com/indeedkeyparser/parse android/app/src/test
git commit -m "feat(android): code+label extraction from ui tree"
```

---

## Task 6: Dedup logic

**Files:**
- Create: `android/app/src/main/java/com/indeedkeyparser/send/Deduper.kt`
- Test: `android/app/src/test/java/com/indeedkeyparser/send/DeduperTest.kt`

**Interfaces:**
- Consumes: `parse.Entry`.
- Produces: `class Deduper(windowMs: Long = 30_000)` with `fun shouldSend(entry: Entry, nowMs: Long): Boolean`.

- [ ] **Step 1: Write the failing test**

```kotlin
package com.indeedkeyparser.send

import com.indeedkeyparser.parse.Entry
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class DeduperTest {
    private val e = Entry("Corp", "123456")

    @Test
    fun sends_first_occurrence() {
        assertTrue(Deduper().shouldSend(e, 0))
    }

    @Test
    fun suppresses_same_within_window() {
        val d = Deduper(windowMs = 30_000)
        assertTrue(d.shouldSend(e, 0))
        assertFalse(d.shouldSend(e, 10_000))
    }

    @Test
    fun sends_again_after_window() {
        val d = Deduper(windowMs = 30_000)
        assertTrue(d.shouldSend(e, 0))
        assertTrue(d.shouldSend(e, 40_000))
    }

    @Test
    fun different_entry_sends() {
        val d = Deduper()
        assertTrue(d.shouldSend(e, 0))
        assertTrue(d.shouldSend(Entry("Bank", "123456"), 1000))
    }
}
```

- [ ] **Step 2: Write `Deduper.kt`**

```kotlin
package com.indeedkeyparser.send

import com.indeedkeyparser.parse.Entry

class Deduper(private val windowMs: Long = 30_000) {
    private var last: Pair<Entry, Long>? = null

    fun shouldSend(entry: Entry, nowMs: Long): Boolean {
        val prev = last
        if (prev != null && prev.first == entry && nowMs - prev.second < windowMs) {
            return false
        }
        last = entry to nowMs
        return true
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cd android && ./gradlew :app:testDebugUnitTest --tests "com.indeedkeyparser.send.DeduperTest"`
Expected: PASS (4 tests).

- [ ] **Step 4: Commit**

```bash
git add android/app/src/main/java/com/indeedkeyparser/send/Deduper.kt android/app/src/test/java/com/indeedkeyparser/send/DeduperTest.kt
git commit -m "feat(android): dedup by (account, code) within time window"
```

---

## Task 7: Android project scaffold + settings storage

Framework glue — verified by building + manual run, not unit tests.

**Files:**
- Create: `android/settings.gradle.kts`, `android/build.gradle.kts`, `android/app/build.gradle.kts`, `android/gradle.properties`
- Create: `android/app/src/main/AndroidManifest.xml`
- Create: `android/app/src/main/java/com/indeedkeyparser/settings/Settings.kt`
- Create: `android/app/src/main/java/com/indeedkeyparser/settings/SettingsActivity.kt`
- Create: `android/app/src/main/res/layout/activity_settings.xml`

**Interfaces:**
- Produces: `class Settings(context)` with `var webhookUrl: String`, `var secret: String` backed by `EncryptedSharedPreferences`.

- [ ] **Step 1: Gradle files.** `android/app/build.gradle.kts` dependencies:

```kotlin
dependencies {
    implementation("androidx.core:core-ktx:1.13.1")
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("androidx.security:security-crypto:1.1.0-alpha06")
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
    implementation("androidx.work:work-runtime-ktx:2.9.1")
    testImplementation("junit:junit:4.13.2")
}
```
Set `minSdk = 26`, `compileSdk = 34`, `applicationId = "com.indeedkeyparser"`, `namespace = "com.indeedkeyparser"`.

- [ ] **Step 2: `Settings.kt`**

```kotlin
package com.indeedkeyparser.settings

import android.content.Context
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey

class Settings(context: Context) {
    private val prefs = run {
        val key = MasterKey.Builder(context)
            .setKeyScheme(MasterKey.KeyScheme.AES256_GCM).build()
        EncryptedSharedPreferences.create(
            context, "settings", key,
            EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
            EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
        )
    }

    var webhookUrl: String
        get() = prefs.getString("webhook_url", "") ?: ""
        set(v) { prefs.edit().putString("webhook_url", v).apply() }

    var secret: String
        get() = prefs.getString("secret", "") ?: ""
        set(v) { prefs.edit().putString("secret", v).apply() }
}
```

- [ ] **Step 3: `SettingsActivity.kt`** — two `EditText`s (URL, secret) bound to `Settings`, a Save button, and a "Enable accessibility" button:

```kotlin
package com.indeedkeyparser.settings

import android.content.Intent
import android.os.Bundle
import android.provider.Settings as AndroidSettings
import android.widget.Button
import android.widget.EditText
import androidx.appcompat.app.AppCompatActivity
import com.indeedkeyparser.R

class SettingsActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_settings)
        val settings = Settings(this)
        val url = findViewById<EditText>(R.id.webhookUrl).apply { setText(settings.webhookUrl) }
        val secret = findViewById<EditText>(R.id.secret).apply { setText(settings.secret) }
        findViewById<Button>(R.id.save).setOnClickListener {
            settings.webhookUrl = url.text.toString().trim()
            settings.secret = secret.text.toString().trim()
        }
        findViewById<Button>(R.id.enableAccessibility).setOnClickListener {
            startActivity(Intent(AndroidSettings.ACTION_ACCESSIBILITY_SETTINGS))
        }
    }
}
```

- [ ] **Step 4: `activity_settings.xml`** — vertical `LinearLayout` with `@+id/webhookUrl`, `@+id/secret` (inputType `textPassword`), `@+id/save`, `@+id/enableAccessibility`.

- [ ] **Step 5: `AndroidManifest.xml`** — register `SettingsActivity` as launcher; declare `INTERNET` permission.

- [ ] **Step 6: Build**

Run: `cd android && ./gradlew :app:assembleDebug`
Expected: BUILD SUCCESSFUL.

- [ ] **Step 7: Commit**

```bash
git add android/settings.gradle.kts android/build.gradle.kts android/gradle.properties android/app
git commit -m "feat(android): project scaffold + encrypted settings screen"
```

---

## Task 8: Webhook sender + WorkManager retry

**Files:**
- Create: `android/app/src/main/java/com/indeedkeyparser/send/SendCodeWorker.kt`
- Create: `android/app/src/main/java/com/indeedkeyparser/send/WebhookSender.kt`
- Test: `android/app/src/test/java/com/indeedkeyparser/send/PayloadTest.kt`

**Interfaces:**
- Consumes: `parse.Entry`, `settings.Settings`.
- Produces:
  - `fun buildPayload(entry: Entry, timestamp: String): String` (JSON matching the contract; pure, unit-tested).
  - `object WebhookSender { fun enqueue(context, entry) }` — builds `timestamp` (RFC3339, UTC) and enqueues `SendCodeWorker` with input data.
  - `class SendCodeWorker(...) : Worker` — reads settings, POSTs the payload with OkHttp and `Authorization: Bearer <secret>`; returns `Result.retry()` on IO/5xx failure, `Result.success()` on 2xx, `Result.failure()` on 4xx.

- [ ] **Step 1: Write the failing test `PayloadTest.kt`**

```kotlin
package com.indeedkeyparser.send

import com.indeedkeyparser.parse.Entry
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class PayloadTest {
    @Test
    fun payload_matches_contract() {
        val json = JSONObject(buildPayload(Entry("Corp", "123456"), "2026-08-31T12:00:00Z"))
        assertEquals("Corp", json.getString("account"))
        assertEquals("123456", json.getString("code"))
        assertEquals("2026-08-31T12:00:00Z", json.getString("timestamp"))
        assertEquals("com.indeedid.key", json.getString("source"))
    }
}
```
(`org.json` is available in Android unit tests via the JRE stub; if not resolved, add `testImplementation("org.json:json:20240303")`.)

- [ ] **Step 2: Write `buildPayload`** in `WebhookSender.kt`

```kotlin
package com.indeedkeyparser.send

import com.indeedkeyparser.parse.Entry
import org.json.JSONObject

fun buildPayload(entry: Entry, timestamp: String): String =
    JSONObject()
        .put("account", entry.account)
        .put("code", entry.code)
        .put("timestamp", timestamp)
        .put("source", "com.indeedid.key")
        .toString()
```

- [ ] **Step 3: Run the test**

Run: `cd android && ./gradlew :app:testDebugUnitTest --tests "com.indeedkeyparser.send.PayloadTest"`
Expected: PASS.

- [ ] **Step 4: Add `SendCodeWorker` and `WebhookSender.enqueue`** (glue; no unit test — verified manually in Task 10):

```kotlin
// append to WebhookSender.kt
import android.content.Context
import androidx.work.*
import com.indeedkeyparser.settings.Settings
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.MediaType.Companion.toMediaType
import java.time.Instant

object WebhookSender {
    fun enqueue(context: Context, entry: Entry) {
        val data = workDataOf(
            "account" to entry.account,
            "code" to entry.code,
            "timestamp" to Instant.now().toString(),
        )
        val req = OneTimeWorkRequestBuilder<SendCodeWorker>()
            .setInputData(data)
            .setConstraints(Constraints(requiredNetworkType = NetworkType.CONNECTED))
            .setBackoffCriteria(BackoffPolicy.EXPONENTIAL, 10, java.util.concurrent.TimeUnit.SECONDS)
            .build()
        WorkManager.getInstance(context).enqueue(req)
    }
}

class SendCodeWorker(ctx: Context, params: WorkerParameters) : Worker(ctx, params) {
    override fun doWork(): Result {
        val settings = Settings(applicationContext)
        val url = settings.webhookUrl
        if (url.isEmpty()) return Result.failure()
        val entry = Entry(inputData.getString("account")!!, inputData.getString("code")!!)
        val ts = inputData.getString("timestamp")!!
        val body = buildPayload(entry, ts).toRequestBody("application/json".toMediaType())
        val request = Request.Builder().url(url)
            .header("Authorization", "Bearer ${settings.secret}")
            .post(body).build()
        return try {
            OkHttpClient().newCall(request).execute().use { resp ->
                when {
                    resp.isSuccessful -> Result.success()
                    resp.code in 500..599 -> Result.retry()
                    else -> Result.failure()
                }
            }
        } catch (e: Exception) {
            Result.retry()
        }
    }
}
```

- [ ] **Step 5: Build**

Run: `cd android && ./gradlew :app:assembleDebug`
Expected: BUILD SUCCESSFUL.

- [ ] **Step 6: Commit**

```bash
git add android/app/src/main/java/com/indeedkeyparser/send android/app/src/test/java/com/indeedkeyparser/send/PayloadTest.kt
git commit -m "feat(android): webhook sender with workmanager retry"
```

---

## Task 9: AccessibilityService wiring

Framework glue — verified manually in Task 10.

**Files:**
- Create: `android/app/src/main/java/com/indeedkeyparser/IndeedCodeAccessibilityService.kt`
- Create: `android/app/src/main/res/xml/accessibility_service_config.xml`
- Modify: `android/app/src/main/AndroidManifest.xml` (register the service)

**Interfaces:**
- Consumes: `parse.{UiNode, extractEntry}`, `send.{Deduper, WebhookSender}`.
- Produces: the running service; a helper `fun toUiNode(info: AccessibilityNodeInfo?): UiNode` converting the framework tree to the testable `UiNode`.

- [ ] **Step 1: `accessibility_service_config.xml`**

```xml
<accessibility-service xmlns:android="http://schemas.android.com/apk/res/android"
    android:accessibilityEventTypes="typeWindowStateChanged|typeWindowContentChanged"
    android:accessibilityFeedbackType="feedbackGeneric"
    android:packageNames="com.indeedid.key"
    android:canRetrieveWindowContent="true"
    android:notificationTimeout="300" />
```

- [ ] **Step 2: `IndeedCodeAccessibilityService.kt`**

```kotlin
package com.indeedkeyparser

import android.accessibilityservice.AccessibilityService
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo
import com.indeedkeyparser.parse.UiNode
import com.indeedkeyparser.parse.extractEntry
import com.indeedkeyparser.send.Deduper
import com.indeedkeyparser.send.WebhookSender

class IndeedCodeAccessibilityService : AccessibilityService() {
    private val deduper = Deduper()

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        val root = rootInActiveWindow ?: return
        val entry = extractEntry(toUiNode(root)) ?: return
        if (deduper.shouldSend(entry, System.currentTimeMillis())) {
            WebhookSender.enqueue(applicationContext, entry)
        }
    }

    override fun onInterrupt() {}

    private fun toUiNode(info: AccessibilityNodeInfo?): UiNode {
        if (info == null) return UiNode(null)
        val children = (0 until info.childCount).mapNotNull { i ->
            info.getChild(i)?.let { toUiNode(it) }
        }
        return UiNode(info.text?.toString(), children)
    }
}
```

- [ ] **Step 3: Register in `AndroidManifest.xml`**

```xml
<service
    android:name=".IndeedCodeAccessibilityService"
    android:permission="android.permission.BIND_ACCESSIBILITY_SERVICE"
    android:exported="false">
    <intent-filter>
        <action android:name="android.accessibilityservice.AccessibilityService" />
    </intent-filter>
    <meta-data
        android:name="android.accessibilityservice"
        android:resource="@xml/accessibility_service_config" />
</service>
```

- [ ] **Step 4: Build**

Run: `cd android && ./gradlew :app:assembleDebug`
Expected: BUILD SUCCESSFUL.

- [ ] **Step 5: Commit**

```bash
git add android/app/src/main/java/com/indeedkeyparser/IndeedCodeAccessibilityService.kt android/app/src/main/res/xml/accessibility_service_config.xml android/app/src/main/AndroidManifest.xml
git commit -m "feat(android): accessibility service reads code and enqueues send"
```

---

## Task 10: End-to-end manual verification

**Files:** Create: `docs/superpowers/notes/2026-08-31-e2e-verification.md`

- [ ] **Step 1: Run the server.**

```bash
cd server && WEBHOOK_SECRET=test-secret DB_PATH=codes.db cargo run
```

- [ ] **Step 2: Install app on device**, open Settings screen, set webhook URL (reachable from the phone, e.g. `http://<host-ip>:8080/webhook`) and secret `test-secret`. Tap Save, then "Enable accessibility" and turn the service on.
- [ ] **Step 3: Open Indeed Key.** Wait for a code to display.
- [ ] **Step 4: Verify receipt.**

```bash
curl -s -H "Authorization: Bearer test-secret" http://localhost:8080/codes
```
Expected: JSON array containing the account + code just shown.

- [ ] **Step 5: Record results** in the notes file (worked / issues / whether the label paired correctly). If pairing is wrong, adjust `extractEntry` per the real tree from the Task 0 spike and re-run Task 5's tests.
- [ ] **Step 6: Commit** the notes.

```bash
git add docs/superpowers/notes/2026-08-31-e2e-verification.md
git commit -m "docs: record end-to-end verification results"
```

---

## Self-Review

- **Spec coverage:** AccessibilityService reading (Tasks 9, 0), extract code + `account` label (Task 5), dedup by `(account, code)` (Task 6), OkHttp POST with bearer (Task 8), WorkManager retry (Task 8), settings + EncryptedSharedPreferences (Task 7), Rust axum server (Tasks 1–4), SQLite storage + idempotent dedup `(account,code,timestamp)` (Task 2), `POST /webhook` + `GET /codes` + auth (Tasks 3–4), env config (Task 1), payload contract (Task 8 + Task 4), spike for FLAG_SECURE/accessibility (Task 0). All spec sections map to a task.
- **Placeholders:** none — every code step has concrete code.
- **Type consistency:** `Entry(account, code)` and `UiNode(text, children)` consistent across Tasks 5/6/8/9; `CodeRecord` fields consistent across Tasks 2/4; `normalize_code`/`valid_account`/`is_authorized` signatures consistent Tasks 3/4.
