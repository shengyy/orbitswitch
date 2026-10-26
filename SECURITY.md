# Security

OrbitSwitch manages the user's own subscription sign-ins. Implementation details live in
[docs/architecture.md](docs/architecture.md).

## What is stored

| Data | Location |
|---|---|
| Antigravity refresh tokens | OS credential store, service `orbitswitch`, keyed by Google account ID |
| Codex / Claude credentials and paired Claude Desktop session | OS credential store, service `orbitswitch.sessions`, keyed by provider/user/workspace in bounded chunks with an atomic generation index |
| Account metadata (provider, email, name, optional photo, plan, binding status, fingerprints) | `accounts.json` in the app data directory |
| In-progress storage transfer (account IDs and credential hashes, never credentials) | Temporary `accounts-migration.json` in the app data directory; removed after completion |
| Quota results and short-lived access tokens | Memory |
| Official clients' credentials | Native files and keychains; rewritten only on a confirmed switch |
| Isolated browser login state | Temporary CLI directory and keychain namespace; removed on completion, cancellation or timeout |
| Logs | App log directory; never contains tokens, cookies or account emails |

Claude Desktop snapshots store only `claude.ai` authentication cookies and auth keys from
`config.json`, never chat history or the full Electron profile.

The app identifier is `io.github.shengyy.orbitswitch`:
- macOS data: `~/Library/Application Support/io.github.shengyy.orbitswitch/`
- macOS logs: `~/Library/Logs/io.github.shengyy.orbitswitch/`
- Windows data: `%APPDATA%\io.github.shengyy.orbitswitch\`

Before normal startup, saved accounts from the earlier `io.github.shengyy.agyorbit` directory and
Antigravity grants from service `agyorbit` are transferred into these current stores. Existing
destination grants take precedence. Source registry and credential entries are removed only after
the destination is durably written and verified. Failed transfers retain resumable state; unrelated
files in the earlier directory are preserved. This does not rewrite any official client's sign-in.

Native client locations are documented in [docs/antigravity-integration.md](docs/antigravity-integration.md)
and [docs/native-integrations.md](docs/native-integrations.md).

## Network destinations

- Google: `accounts.google.com`, `oauth2.googleapis.com`, `www.googleapis.com`,
  `daily-cloudcode-pa.googleapis.com` (with the locally installed Antigravity version).
- OpenAI: official CLI/browser sign-in, `auth.openai.com` for inactive token refresh,
  `chatgpt.com/backend-api/wham/usage` for quota and workspace verification.
- Anthropic: official CLI/browser sign-in, `platform.claude.com/v1/oauth/token` for inactive refresh,
  `api.anthropic.com/api/oauth/profile` and `/usage`, and `claude.ai/api/bootstrap` for paired
  desktop session verification.
- GitHub Releases: update metadata and confirmed installer downloads, verified against the embedded
  updater public key and signed version.

The shared HTTP client refuses redirects. The frontend receives only metadata and quota snapshots.
Antigravity's OAuth client secret is read from the local installation at runtime and never stored in
this repository.

## Reporting

Use [private vulnerability reporting](https://github.com/shengyy/orbitswitch/security/advisories/new)
with version, platform and reproduction steps using fictional data.
