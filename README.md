# indeed-key-parser

Android app + Rust server that reads your **own** one-time codes (OTP) from the
**Indeed Key** app (`com.indeedid.key`) on your own device and forwards each
code — with the account/token label — to a self-hosted webhook that stores it.

This is personal automation for your own codes on your own device. It is **not**
a covert/exfiltration tool: the reader is an Android AccessibilityService that
**you** enable by hand, and the app reads the code only after you tap a token in
Indeed Key to reveal it.

## Repository layout

- `android/` — the Kotlin app (AccessibilityService reader + settings screen).
- `server/` — the Rust webhook server (`axum` + SQLite).

## Install (from a release)

The signed APK is attached to each GitHub release — no build tools needed.

1. Download `indeed-key-parser-<version>.apk` from the
   [latest release](https://github.com/danilpavlov/indeed-key-parser/releases/latest).
2. On the phone, allow installing from your browser/messenger:
   **Settings → Apps → Special access → Install unknown apps**.
3. Open the APK and install. Requires Android 8.0+ (`minSdk 26`).

Then set it up:

1. **Run the server** (see below) with a `WEBHOOK_SECRET`.
2. Open the app, pick a **Mode** (see below), fill in its field(s) and the
   **Secret**, tap **Save**.
3. Tap **Enable accessibility** and turn the service on for "Indeed Key Parser".
4. Open Indeed Key and tap a token to reveal its code — it is forwarded to your
   server. Read them back with `GET /codes`.

### Modes

The app sends to one of two targets, chosen on the settings screen:

- **Direct (USB)** — the phone is plugged into the machine running the server.
  The app posts to `http://localhost:<port>`, tunnelled to the host over USB, so
  no phone network is needed (delivery does not wait for a connection). On the
  host, forward the port once per connection:
  ```bash
  adb reverse tcp:8080 tcp:8080   # port must match the app's Port field
  ```
  > **Caution:** loopback does not authenticate the listener. Keep `adb reverse`
  > active whenever Direct mode is on — if the tunnel is down, any other app
  > listening on that port on the phone would receive the code and the Bearer
  > secret. Treat the webhook secret as rotatable, and prefer Remote mode with
  > `https://` when you cannot guarantee the tunnel.
- **Remote** — the server is a LAN device or a VPS the phone reaches over the
  network. The app posts to the **Webhook URL** you enter and requires an active
  connection. `http://` LAN webhooks work out of the box (cleartext is
  permitted); `https://` also works and stays certificate-validated — prefer it
  when the server has a trusted certificate.

Verify a download against the `SHA-256` printed in the release notes. All
releases are signed with the same certificate (`CN=Indeed Key Parser`).

## Run the server

```bash
cd server
WEBHOOK_SECRET=<your-secret> BIND_ADDR=0.0.0.0:8080 DB_PATH=codes.db cargo run --release
```

- `POST /webhook` — stores a code. Body:
  `{"account","code","timestamp","source"}`; `Authorization: Bearer <secret>`.
- `GET /codes` — returns recent codes (newest first); same `Bearer` auth.

```bash
curl -s -H "Authorization: Bearer <your-secret>" http://localhost:8080/codes
```

### With Docker

```bash
cp .env.example .env      # then edit .env and set WEBHOOK_SECRET
docker compose up -d      # builds the image and starts the server on :8080
```

The database is kept in the `webhook-data` named volume, so codes survive
restarts and rebuilds. Stop with `docker compose down` (add `-v` to also delete
the stored codes).

## Build from source

```bash
# Server
cd server && cargo test && cargo build --release

# App
cd android
echo "sdk.dir=$HOME/Android/Sdk" > local.properties   # adjust to your SDK path
./gradlew :app:testDebugUnitTest        # unit tests
./gradlew :app:assembleDebug            # debug APK (debug-signed)
./gradlew :app:assembleRelease          # release APK (see signing below)
```

## Signing key

Release APKs are signed with a keystore that is **not** in this repository.
Android requires every update to an installed app to be signed with the **same**
key, so this keystore must be preserved for the life of the app.

- The keystore (`android/release.jks`) and its passwords
  (`android/keystore.properties`) are kept local and are git-ignored — they must
  never be committed.
- `app/build.gradle.kts` reads `keystore.properties` at build time. When the
  file is absent (e.g. a fresh clone or CI without the secrets),
  `assembleRelease` produces an **unsigned** APK instead of failing.

`keystore.properties` format:

```properties
storeFile=release.jks
storePassword=<store password>
keyAlias=<alias>
keyPassword=<key password>
```

To create a fresh keystore (only for a brand-new app identity — a new key cannot
update apps installed with the old one):

```bash
cd android
keytool -genkeypair -v -keystore release.jks -alias indeedkeyparser \
  -keyalg RSA -keysize 2048 -validity 10000
```

**Back up `release.jks` and its passwords** somewhere safe (a password manager or
a private, offline backup). If the key is lost, you cannot ship updates to anyone
who already installed the app — they would have to uninstall and reinstall a
differently-signed build.

## Continuous integration

`.github/workflows/android.yml` runs on every push and pull request to `master`:
it runs the unit tests and builds the debug APK (uploaded as a build artifact).
On a version tag (`v*`) it additionally builds a **signed** release APK and
attaches it to the GitHub Release for that tag.

The push/PR build needs no secrets. The signed-release job only runs on a tag,
and if the signing secrets are not set it skips itself (the run stays green) —
so you can ignore signing entirely and just use the debug APK artifact. To
enable signed releases, add these repository secrets (Settings → Secrets and
variables → Actions):

| Secret | Value |
|--------|-------|
| `KEYSTORE_BASE64` | the keystore, base64-encoded: `base64 -w0 android/release.jks` |
| `KEYSTORE_PASSWORD` | the store password from `keystore.properties` |
| `KEY_ALIAS` | the key alias (e.g. `indeedkeyparser`) |
| `KEY_PASSWORD` | the key password from `keystore.properties` |

The workflow decodes the keystore and writes `keystore.properties` at build time;
neither the key nor the passwords are stored in the repository. To cut a release,
bump `versionCode`/`versionName` in `app/build.gradle.kts`, then push a tag:

```bash
git tag v1.1.0 && git push origin v1.1.0
```
