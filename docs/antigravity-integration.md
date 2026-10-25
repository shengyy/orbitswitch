# Antigravity integration

This page owns Antigravity's external contracts: credentials, OAuth, APIs and processes, with their
verified versions. Unchecked platform equivalents are marked *not verified*. OrbitSwitch's adapter
implementation lives in [architecture.md](architecture.md#antigravity-adapters); the verification
procedure is in [CONTRIBUTING.md](../CONTRIBUTING.md#verifying-against-a-real-antigravity).

| Fact | Verified on | Owner module |
|---|---|---|
| Signed-in credential storage | 2.19.1, macOS 27 | `antigravity/live.rs` |
| Credential bundle format | 2.19.1, macOS 27 | `antigravity/credential.rs` |
| OAuth client and scopes | 2.19.1, macOS 27 | `antigravity/oauth_client.rs`, `google/oauth.rs` |
| Installed app version metadata | 2.23.1, macOS 27 | `antigravity/version.rs` |
| Quota and plan endpoints | 2.19.1, 2.22.0 and 2.23.1, macOS 27 | `google/cloudcode.rs` |
| Processes that hold the credential | 2.19.1, macOS 27 | `antigravity/process.rs` |
| Windows equivalents | **not verified** | same modules |

## Signed-in credential

Antigravity's Go language server stores the credential with
[go-keyring](https://github.com/zalando/go-keyring) under service `gemini`, account `antigravity`:

- **macOS**: a generic password in the login keychain, written through `/usr/bin/security`. The value is
  `go-keyring-base64:` followed by base64 of the JSON bundle.
- **Windows** (from go-keyring's documented layout, not yet verified): a generic credential with target
  `gemini:antigravity` whose blob is the raw JSON.

It also mirrors the JSON to `~/.gemini/jetski-standalone-oauth-token` (mode `0600`) and reads that file
when the keyring times out (the binary logs `Keyring LoadStoredToken timed out … falling back to file
storage`). OrbitSwitch's read/write strategy is in
[Credential access](architecture.md#credential-access).

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

- `token` is golang.org/x/oauth2's `Token`; `expiry` uses Go's RFC 3339 format with the local offset.
- `auth_method` is `consumer` for personal Google accounts. Workspace / Cloud sign-ins are untested.
- `id_token` identifies the account (`sub`, `email`). A refresh-token grant returns a new one.

Bundle handling during a switch is described in [Credential access](architecture.md#credential-access).

## OAuth client

The consumer sign-in client is `1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com`
(the `aud` of Antigravity's ID tokens). The binary also embeds a second client used for Google Cloud
sign-in (`884354919052-…`), which OrbitSwitch does not use.

The client ID is public. The installed-app secret is embedded in the local binaries and is **not**
stored in this repository. A token request with a bogus authorization code fails with `invalid_grant`
for the matching secret; a wrong secret fails with `invalid_client`. OrbitSwitch's scan and selection are
described in [OAuth client discovery](architecture.md#oauth-client-discovery).

Facts the browser sign-in depends on:

- The client is a desktop client: Google accepts `http://127.0.0.1:<any port>/…` redirects.
- Requested scopes are exactly what the IDE's own sign-in is granted (read back via Google's tokeninfo):
  `openid email profile` plus `cloud-platform`, `aicode`, `cclog` and `experimentsandconfigs` under
  `https://www.googleapis.com/auth/`. A token from OrbitSwitch is therefore interchangeable with the IDE's.
- `prompt=select_account consent` always shows the account chooser and always returns a refresh token.

## Quota and plan

Base URL: `https://daily-cloudcode-pa.googleapis.com/v1internal` (what Antigravity passes to its language
server as `--cloud_code_endpoint`). Both calls are `POST` with a bearer token.

- **The `User-Agent` must include the installed Antigravity product/version token.** OrbitSwitch sends
  `OrbitSwitch/<version> (antigravity/<installed-version>)`. Read-only checks with 2.23.1 showed that
  `OrbitSwitch/<version> (antigravity)` receives HTTP 200 with only Claude/GPT; including `antigravity/2.23.1`
  restores Gemini and matches the official app's freshly fetched quota summary. An unversioned token
  or using OrbitSwitch's own version as the Antigravity version also omitted Gemini. Without an
  Antigravity identity, the API can answer 403 "You do not have a valid license of this product".
- `:retrieveUserQuotaSummary` with `{}` returns `groups[]`, each with `displayName`, a `description` that
  lists the models (`Models within this group: Claude Opus, Claude Sonnet, GPT-OSS`) and `buckets[]` with
  `window` (`5h` or `weekly`), `remainingFraction` and `resetTime`. Observed groups: `Gemini Models` and
  `Claude and GPT models`. The response is proto3 JSON, so a fraction of `0` is **omitted**: a bucket
  without `remainingFraction` is exhausted.
- The versioned identity returned both groups for six Pro accounts and one Ultra account with
  2.23.1. These responses and the official app's fresh summary did not expose a separate Gemini 4
  Argon group; its independent quota contract is **not verified**.
- `:loadCodeAssist` with `{"metadata":{"ideType":"ANTIGRAVITY"}}` returns `paidTier` (`g1-pro-tier` /
  "Google AI Pro", Ultra contains `ultra`) and `currentTier`. `currentTier` can be `free-tier` even
  with a paid subscription, so `paidTier` takes precedence. `paidTier` can itself be `free-tier` /
  "Antigravity Starter Quota"; the field's presence does not imply a paid subscription.
- An account whose Pro subscription had expired returned HTTP 200 with `paidTier.id = free-tier`
  from `:loadCodeAssist`, while `:retrieveUserQuotaSummary` returned HTTP 403. The two results must be
  handled independently; a quota 403 alone does not identify the subscription tier.
- The Free and active Ultra responses checked with 2.22.0 contained no subscription expiry or renewal
  date. These calls provide the current tier and quota windows, not a billing end date; quota reset
  times and OAuth token expiry are not subscription expiry dates.

### Installed app version

The Electron package stores its version in the root `package.json` inside `Resources/app.asar`
on macOS. It matched the bundle's `CFBundleShortVersionString` on 2.23.1. OrbitSwitch reads only that
entry, without launching Antigravity, and refreshes the identity for each quota/plan pass. If the
version cannot be read, the lookup fails instead of sending an identity known to omit pools.
The corresponding Windows `resources/app.asar` layout is **not verified** on a real installation.

## Processes

Every process that may hold the credential has to exit before a switch, or it could write the previous
account back when it refreshes its token:

- the app: `Antigravity.app/Contents/MacOS/Antigravity` (Windows: `Antigravity.exe`);
- its Electron helpers and the language server(s) under the same install root
  (`Contents/Resources/bin/language_server --standalone …`);
- the `agy` CLI.

OrbitSwitch's stop and launch mechanism is in [Process control](architecture.md#process-control).
Each new language server creates an empty file in `~/.gemini/antigravity/crashes/`; that is a marker,
not a crash.

## Not touched

`~/Library/Application Support/Antigravity/app_storage.json` keeps
`jetski.onboarding.lastLoginUsername`, which only pre-fills Antigravity's own sign-in screen. OrbitSwitch
leaves it alone.
