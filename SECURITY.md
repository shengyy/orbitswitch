# Security

OrbitSwitch manages the user's own subscription sign-ins. This page owns storage, network destinations
and vulnerability reporting. [Architecture](docs/architecture.md) owns implementation mechanisms.

## What is stored

| Data | Location |
|---|---|
| Antigravity refresh tokens | OS credential store, service `agyorbit`, keyed by Google account ID |
| Codex / Claude credentials and paired Claude Desktop authentication session | OS credential store, service `orbitswitch.sessions`, provider/user/workspace keys; bounded chunks and an atomic generation index |
| Account identity, provider, email, name, optional photo, plan, binding status and fingerprints | `accounts.json` in the app data directory |
| Quota results and transient access-token cache | Memory |
| Official clients' credentials | Their native files/keychains; rewritten only on a confirmed switch |
| Temporary browser login credentials | A private isolated CLI directory and its keychain namespace; cleaned after completion, cancellation or timeout |
| Logs | App log directory, without tokens, cookies or account emails |

Native session snapshots contain access tokens as well as refresh tokens. Claude Desktop snapshots
contain only authentication cookies/configuration, not a copied browser profile or chat archive.
A process crash during an isolated login can leave a temporary directory; the changelog explains
manual removal. No migration routine scans or rewrites old client state.

The stable app identity is `io.github.shengyy.agyorbit`: macOS app data lives at
`~/Library/Application Support/io.github.shengyy.agyorbit/` and logs at
`~/Library/Logs/io.github.shengyy.agyorbit/`. Windows data lives at
`%APPDATA%\io.github.shengyy.agyorbit\`. The retained identifier preserves existing accounts and
login-item identity after the product rename. Native locations are documented in the
[Antigravity](docs/antigravity-integration.md) and [Codex/Claude](docs/native-integrations.md) contracts.

## Network destinations

- Google: `accounts.google.com`, `oauth2.googleapis.com`, `www.googleapis.com`,
  `daily-cloudcode-pa.googleapis.com` for official sign-in, refresh, identity, plan and quota.
  Cloud Code requests include the locally installed Antigravity version.
- OpenAI: the official CLI/browser authentication flow, `auth.openai.com` for inactive-token refresh,
  `chatgpt.com/backend-api/wham/usage` for subscription quota and workspace verification.
- Anthropic: the official CLI/browser sign-in flow, `platform.claude.com/v1/oauth/token` for inactive
  refresh, `api.anthropic.com/api/oauth/profile` and `/usage`, and `claude.ai/api/bootstrap` for
  validating the paired desktop session. Cookie headers are used only on that Claude web endpoint.
- GitHub Releases: update metadata and confirmed installer downloads. No account credentials are
  attached. Tauri verifies package signatures and signed versions before installation.

The shared authenticated HTTP client refuses redirects. The frontend receives metadata and quota
snapshots, never credentials, OAuth responses or desktop cookies. There is no OrbitSwitch server,
model proxy, analytics or telemetry. Never submit real accounts or credential files in public issues.
Antigravity's client secret is discovered from the local official installation, never redistributed.

## Reporting

Use [private vulnerability reporting](https://github.com/shengyy/orbitswitch/security/advisories/new).
Include version, platform and reproducible steps with fictional data.
