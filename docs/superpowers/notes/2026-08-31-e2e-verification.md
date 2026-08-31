# E2E verification (Task 10)

**Date:** 2026-08-31
**Device:** POCO C71 (25028PC03G), Android 15 (SDK 35), HyperOS.
**Result:** ✅ Full pipeline confirmed end to end.

## Transport chosen: USB loopback (adb reverse)

The device had no usable LAN path to the dev machine: Wi-Fi client was off
(the phone was running its own hotspot "POCO C71"), and the only active
network was a dead AmneziaWG VPN (tun0, no underlying network). So instead of
LAN IP we tunnelled over USB:

```
adb reverse tcp:8080 tcp:8080      # phone localhost:8080 -> host:8080
```

- App webhook URL: `http://localhost:8080/webhook`
- Cleartext for `localhost`/`127.0.0.1` (and `192.168.1.47`) is whitelisted by
  the scoped `res/xml/network_security_config.xml`; everything else stays
  cleartext-blocked (app default at targetSdk 34).
- WorkManager's `NetworkType.CONNECTED` constraint was satisfied by the VPN
  network, and the POST reached the host through the loopback tunnel.

## Steps run

1. `./gradlew :app:assembleDebug`, `adb install -r app-debug.apk`.
2. `adb reverse tcp:8080 tcp:8080`.
3. Server: `WEBHOOK_SECRET=e2e-secret-123 BIND_ADDR=127.0.0.1:8080 DB_PATH=… ./indeed-key-webhook`.
4. Settings (filled via adb `input`): webhook `http://localhost:8080/webhook`,
   secret `e2e-secret-123`, Save.
5. Accessibility service enabled without root via secure settings:
   `settings put secure enabled_accessibility_services com.indeedkeyparser/…IndeedCodeAccessibilityService`
   `settings put secure accessibility_enabled 1`.
   Confirmed bound in `dumpsys accessibility` (Bound services: Indeed Key Parser).
6. Opened Indeed Key, tapped the token to reveal/copy the code.

## Observations

- `GET /codes` returned exactly the expected record:
  ```json
  {"account":"test1","code":"388471","timestamp":"2026-08-31T10:44:55.890759Z","source":"com.indeedid.key"}
  ```
  Account label correct (`tName`), code normalised (space stripped), source and
  RFC3339 timestamp correct.
- **Dedup:** re-tapping the same token (same code, within the 30 s window) did
  NOT create a duplicate — still one record.
- **Auth:** `GET /codes` with a wrong bearer → `401`.

## Notes for real (non-test) use

- The `adb reverse` tunnel and the test server are transient. For everyday use
  the user needs a persistent server on a host the phone can actually reach and
  a webhook URL for that host; if it is an `http://` LAN IP, add that IP to
  `network_security_config.xml` (or serve over HTTPS, which the app already
  validates).
- Re-point the app's Settings (webhook URL + secret) away from the test values.
- The accessibility service was enabled here via adb; on the device the user
  toggles it under Settings → Accessibility.
