# Indeed Key Code Parser — Design

**Date:** 2026-08-31
**Status:** Approved (design), pending implementation plan

## Goal

Read the user's **own** one-time codes (OTP) from the Indeed Key app
(`com.indeedid.key`) screen on their own Android device, and forward each
code — together with the name of the token/account that generated it — to a
self-hosted webhook that stores the codes.

Scope is a single user reading their own codes on their own device. Not a
covert/exfiltration tool: enabling the reader requires an explicit,
user-performed Android Accessibility opt-in.

## Components

### 1. Android app (Kotlin)

Reads the code from the Indeed Key UI tree via `AccessibilityService`
(no OCR — the "path 1" approach) and POSTs it to the webhook.

- **`IndeedCodeAccessibilityService : AccessibilityService`**
  - Subscribes to window events for package `com.indeedid.key` only
    (`TYPE_WINDOW_STATE_CHANGED` + `TYPE_WINDOW_CONTENT_CHANGED`).
  - On event: walks `rootInActiveWindow`, collects text nodes.
- **`extractEntry(nodes): Entry?`** — pure function.
  `Entry = { account: String, code: String }`.
  Finds a node whose text matches an OTP code (`\b\d{6,8}\b`) and pairs it
  with the nearest label node (account/token name) in the same list-row /
  container. Kept as a pure function so it is unit-testable without a device.
- **Dedup** — do not resend the same `(account, code)` pair; keep last sent
  pair plus a time window.
- **Sender** — OkHttp `POST` JSON to the webhook with header
  `Authorization: Bearer <secret>`. Offline resilience via **WorkManager**
  (retries with backoff when there is no network).
- **Settings screen** — one Activity: fields "Webhook URL" and "Secret",
  stored in `EncryptedSharedPreferences`; a button "Enable accessibility"
  that opens the system Accessibility settings (the user must enable the
  service manually — Android requires this).

No separate foreground service: the `AccessibilityService` runs in the
background while enabled (YAGNI).

### 2. Webhook server (Rust)

- Stack: **`axum` + `tokio`**; storage: **SQLite** via `sqlx`.
- Endpoints:
  - `POST /webhook` — checks `Bearer`; body `{account, code, timestamp,
    source}`; validates (`account` non-empty, `code` matches `\d{6,8}`);
    inserts into DB.
  - `GET /codes` — checks `Bearer`; returns the last N codes (JSON; optional
    simple HTML page later).
- Config via env: `WEBHOOK_SECRET`, `BIND_ADDR`, `DB_PATH`.

## Payload contract

```json
{
  "account": "My Corp Account",
  "code": "123456",
  "timestamp": "2026-08-31T12:00:00Z",
  "source": "com.indeedid.key"
}
```

- `account` — name of the token/component that generated the code (visible in
  the Indeed Key UI next to the code).

## Data flow

Indeed Key shows a code -> Accessibility event -> service reads text nodes ->
`extractEntry` (code + account) -> dedup by `(account, code)` -> OkHttp POST ->
Rust server checks `Bearer` -> insert into SQLite -> viewable via `GET /codes`.

## Error handling

- App: no network -> WorkManager queue; server error -> retry with backoff;
  parse failure -> skip silently (log only).
- Server: bad `Bearer` -> `401`; malformed body -> `400`; duplicate
  `(account, code, timestamp)` -> `200` (idempotent).

## Auth

Shared secret between app and server: `Authorization: Bearer <secret>`.
Required on both `POST /webhook` and `GET /codes`.

## Testing

- Rust: unit tests for auth middleware and handlers (valid / invalid /
  duplicate) + an integration test through a test client.
- Kotlin: unit tests for `extractEntry(...)` against fixed sample UI trees.

## First implementation step — spike

Before building the full app, verify on a real device with Indeed Key
installed:

1. Is the **code** exposed through the accessibility tree (not blocked)?
2. Is the **account label** present nearby, and how is it related to the code
   node in the tree (same row / parent container)?

If the code is not exposed to accessibility, revisit "path 2" (MediaProjection
+ OCR). All other work proceeds only after the spike confirms path 1.

## Repository layout

Monorepo:

- `android/` — the Kotlin app.
- `server/` — the Rust webhook server.
- `docs/` — specs and notes.
