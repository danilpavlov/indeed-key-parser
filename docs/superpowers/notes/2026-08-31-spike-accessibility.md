# Spike (Task 0) — Indeed Key accessibility readout

**Date:** 2026-08-31
**Device:** POCO C71 (model 25028PC03G), Android 15 (SDK 35), HyperOS.
**App:** `com.indeedid.key`, activity `com.general.key.ui.MainActivity`.
**Result:** ✅ Path 1 (AccessibilityService reads the code as text) is viable.
No `FLAG_SECURE` blocking on the authenticator list — `uiautomator dump`
returns the tree and the code text once revealed.

## What the tree looks like

Each authenticator row exposes stable, prefixed resource-ids (`elN-` per row):

| resource-id        | example value              | role                     |
|--------------------|----------------------------|--------------------------|
| `elN-iAuthenticator` | (container)              | row container            |
| `elN-tName`        | `test1`                    | token name  → `account`  |
| `elN-tAccount`     | `dsds`                     | account / issuer sublabel|
| `elN-bOpen`        | (button)                   | reveal/copy control      |
| `elN-tOTP`         | `••• •••` → `851 589`      | the OTP code             |
| `elN-progress`     | (progress bar)             | TOTP countdown           |
| `elN-tTapHint`     | `Коснитесь для копирования`| tap hint                 |

Nav chrome: `bNavSettings`, `bNavAdd`, `tNavTitle` (`Аутентификаторы`).

## Key behavioural findings

1. **Code is masked by default.** `elN-tOTP` reads `••• •••` (6 dots, 3+3
   grouping → 6-digit code) until the user taps the row.
2. **Tap reveals the code in the tree.** Tapping the row copies the code to
   the clipboard (toast: "Код для test1 был скопирован") AND flips
   `elN-tOTP` to the real digits, e.g. `851 589` (6 digits, space-separated).
   The reveal persists on the list afterwards. → capture is **tap-triggered**:
   the user's normal "tap to copy" gesture is what surfaces the code, which
   the AccessibilityService then reads via `TYPE_WINDOW_CONTENT_CHANGED`.
   (Immediately after the tap a transient state returns `null root node`;
   a retry a moment later dumps cleanly — the service should tolerate this.)
3. **Code format:** `NNN NNN` — digits with an embedded space. Both the app's
   `normalize`/`isCode` and the server's `normalize_code` strip whitespace, so
   `851 589` → `851589`. Consistent with the 6–8 digit contract.
4. **Two candidate labels per row.** `tName` ("test1") is the token name;
   `tAccount` ("dsds") is a secondary issuer field. We use `tName` as the
   payload `account`, falling back to `tAccount` when `tName` is empty.
5. **Tree shape.** uiautomator serialises the row as a nested chain
   (tName > tAccount > bOpen > tOTP > progress > tTapHint), not flat siblings.
   The original positional `extractEntry` (code + label as *direct children*
   of one container) does not match this shape reliably → switched to
   resource-id keyed extraction (`elN-tOTP` for the code, `elN-tName`/
   `elN-tAccount` for the label), with the positional logic kept as a fallback.

## Consequence for the implementation

- `UiNode` gains `viewId` (from `AccessibilityNodeInfo.viewIdResourceName`).
- `IndeedCodeAccessibilityService.toUiNode` captures that id.
- `extractEntry` tries resource-id extraction first, then falls back to the
  structural pairing (keeps generic behaviour + existing tests green).
