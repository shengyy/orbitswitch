# Antigravity integration

External contracts for Google Antigravity: credentials, OAuth, Cloud Code APIs and processes. Adapter
internals live in [architecture.md](architecture.md); read-only verification commands live in
[CONTRIBUTING.md](../CONTRIBUTING.md#read-only-checks-against-installed-clients).

| Contract | Verified on | Owner module |
|---|---|---|
| Signed-in credential storage | 2.19.1–2.26.0, macOS 27 | `antigravity/live.rs` |
| Credential bundle format | 2.19.1–2.26.0, macOS 27 | `antigravity/credential.rs` |
| OAuth client and scopes | 2.19.1–2.26.0, macOS 27 | `antigravity/oauth_client.rs`, `google/oauth.rs` |
| Installed version in `app.asar` | 2.23.1–2.26.0, macOS 27 | `antigravity/version.rs` |
| Quota and plan endpoints | 2.19.1–2.26.0, macOS 27 | `google/cloudcode.rs` |
| Processes holding the credential | 2.19.1–2.26.0, macOS 27 | `antigravity/process.rs` |
| Windows equivalents | **not verified** | same modules |

## Signed-in credential

Antigravity's Go language server stores the active credential with
[go-keyring](https://github.com/zalando/go-keyring) under service `gemini`, account `antigravity`:

- **macOS**: generic password in the login keychain via `/usr/bin/security`, formatted as
  `go-keyring-base64:<base64-json>`.
- **Windows** (**not verified**): generic credential with target `gemini:antigravity` containing raw
  UTF-8 JSON.

It also mirrors the raw JSON to `~/.gemini/jetski-standalone-oauth-token` (mode `0600`) as a timeout
fallback. OrbitSwitch reads the keyring first and writes both locations on switch.

## Credential bundle

```json
{
  "token": {
    "access_token": "…",
    "token_type": "Bearer",
    "refresh_token": "…",
    "expiry": "2026-10-03T17:52:30.076579+08:00"
  },
  "auth_method": "consumer",
  "id_token": "…"
}
```

- `token` matches `golang.org/x/oauth2.Token`; `expiry` uses RFC 3339 with the local offset.
- `auth_method` is `consumer` for personal Google accounts.
- `id_token` carries `sub` and `email`. Unknown fields are preserved on round-trip.

## OAuth client

- Consumer client ID: `1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com`.
- Client secret: scanned at runtime from `Contents/Resources/bin/` (35-byte `GOCSPX-` candidates) and
  validated against `https://oauth2.googleapis.com/token` (`invalid_grant` identifies the matching
  secret; `invalid_client` rejects wrong candidates).
- Redirect URI: loopback `http://127.0.0.1:<ephemeral-port>/oauth-callback`.
- Scopes: `openid`, `email`, `profile`, and `cloud-platform`, `aicode`, `cclog`,
  `experimentsandconfigs` under `https://www.googleapis.com/auth/`.
- `prompt=select_account consent` forces account selection and issues a refresh token.

## Quota and plan

Base URL: `https://daily-cloudcode-pa.googleapis.com/v1internal` (`POST` with bearer token).

- **`User-Agent`**: `OrbitSwitch/<version> (antigravity/<installed-version>)`, where `<installed-version>`
  is read from the root `package.json` inside `Resources/app.asar` on each refresh. An unversioned or
  stale product token causes `:retrieveUserQuotaSummary` to omit the Gemini group; missing the
  `antigravity` token entirely returns HTTP 403.
- **`:retrieveUserQuotaSummary`** (`{}`): returns `groups[]` with `displayName`, `description`
  (`Models within this group: …`) and `buckets[]` (`window`: `5h` or `weekly`, `remainingFraction`,
  `resetTime`). Proto3 JSON omits `0`, so a bucket without `remainingFraction` is exhausted (`0.0`).
- **`:loadCodeAssist`** (`{"metadata":{"ideType":"ANTIGRAVITY"}}`): returns `paidTier` and
  `currentTier`. `paidTier` takes precedence (`ultra` → Ultra, `pro` → Pro, `free-tier` or absent →
  Free). Plan and quota responses are handled independently so an expired subscription updates to
  `Free` even when `:retrieveUserQuotaSummary` returns HTTP 403.

## Processes

All processes that hold the active credential must exit before rewriting it:

- `Antigravity.app/Contents/MacOS/Antigravity` (Windows: `Antigravity.exe`)
- Electron helpers and `Contents/Resources/bin/language_server` under the install root
- `agy` CLI

`~/Library/Application Support/Antigravity/app_storage.json` (`jetski.onboarding.lastLoginUsername`)
is not modified.
