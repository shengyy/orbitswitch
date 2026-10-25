# Native subscription integrations

This page owns Codex and Claude credential, identity and quota contracts. Antigravity has its
[own contract](antigravity-integration.md). [Status](status.md) records the verification boundary.

## Evidence and references

Implementations were checked against these primary sources, rather than inferring behavior from
product names. Reference code has not been vendored.

| Source | What it establishes |
|---|---|
| [OpenAI auth storage](https://github.com/openai/codex/blob/main/codex-rs/login/src/auth/storage.rs) | `CODEX_HOME`, file / direct keyring / auto storage, canonical-path key hashing |
| [codex-auth](https://github.com/Loongphy/codex-auth) | Preserve native `auth.json`; query the Codex usage endpoint |
| [codex-account-switcher](https://github.com/liuzhao1225/codex-account-switcher) | Close desktop, save outgoing credential, activate, verify and reopen; bounded restoration |
| [claude-swap](https://github.com/realiti4/claude-swap) | CLI keychain / file credentials plus `oauthAccount`; active-client refresh ownership; preserve rotated tokens |
| [CodexBar](https://github.com/steipete/CodexBar) | Native usage/profile contracts; distinguish missing windows, scopes, stale data and account ownership |
| [Claude-Code-Desktop-Switcher](https://github.com/PriyanshuGeTRekT/Claude-Code-Desktop-Switcher) | Desktop uses independent Electron profile storage, not the CLI credential alone |

Local read-only probes cover Codex CLI 0.164.0, Codex desktop 26.1021.31345 (bundle
`com.openai.codex`, installed as `ChatGPT.app`), Claude Code 2.1.310 and Claude Desktop 3.6556.2.
macOS discovery verifies the bundle identifier and executable in `Info.plist`; a file named
`ChatGPT.app` alone is not evidence that it is the Codex client.
Source inspection is not an end-to-end switch test. See [status](status.md).

## Codex

- Default credential: `~/.codex/auth.json`; `CODEX_HOME` selects another CLI home.
- `config.toml` selects `cli_auth_credentials_store`. Supported modes are `file`, direct `keyring`
  and `auto`; unknown / ephemeral modes fail closed. Custom desktop homes are rejected for switching.
- Direct keyring service is `Codex Auth`; account key is `cli|` plus the first 16 hexadecimal characters
  of SHA-256 of the canonical home path. Use the upstream `keyring` crate on both operating systems:
  macOS direct storage avoids the system command's short interactive-line limit; Windows uses Codex's
  credential target convention instead of Antigravity's. New encrypted-secret backends are not verified.
- Preserve the complete credential object, including ID, access and refresh tokens. Identity is the
  user ID from ID-token claims plus `tokens.account_id` for the workspace. Email is display data.
- `GET https://chatgpt.com/backend-api/wham/usage` uses the access token and `ChatGPT-Account-Id`.
  Its returned user/workspace IDs must match before attaching quota or applying a sign-in.
- `rate_limit`, `code_review_rate_limit` and `additional_rate_limits` carry independent windows.
  Preserve `limit_window_seconds`; remaining share is `1 - used_percent / 100`. Read `plan_type`.
  Window duration, not `primary_window` / `secondary_window` position or plan name, determines the
  label. A weekly-only response produces only a weekly meter. No five-hour window is synthesized.
  Recommendations compare accounts with matching reported window durations. The
  [official plan guide](https://learn.chatgpt.com/docs/pricing) currently distinguishes Pro without
  five-hour limits from Plus and Standard Business; the API response remains the source of truth.
- Inactive-token refresh uses the official public OAuth client and `https://auth.openai.com/oauth/token`.
  No internal-only `chatgptAuthTokens` app-server protocol is used.

## Claude Code

- Default file credential: `~/.claude/.credentials.json`. On macOS the native keychain entry
  `Claude Code-credentials` takes precedence. Locked or unreadable keychains are errors, not permission
  to use a potentially stale file. Windows uses the credential file.
- `CLAUDE_CONFIG_DIR` changes the configuration root. `CLAUDE_SECURESTORAGE_CONFIG_DIR` overrides the
  keychain namespace; nonempty custom namespaces use the first eight SHA-256 hexadecimal characters
  of the NFC-normalized string as a service suffix. No canonicalization changes that string.
- Switch only `claudeAiOauth` in credentials and `oauthAccount` in `.claude.json`, preserving unrelated
  settings. The default metadata file is `~/.claude.json`; custom roots keep it inside the root.
- `GET https://api.anthropic.com/api/oauth/profile` establishes account UUID and organization UUID.
  `GET https://api.anthropic.com/api/oauth/usage` returns shared windows and scoped model limits.
  Both use bearer authentication and `anthropic-beta: oauth-2025-04-20`.
- Remaining share is `1 - utilization / 100`. Prefer active `weekly_scoped` entries over duplicate
  legacy Sonnet/Opus windows. Missing or null values stay unavailable. Subscription names retain
  unknown provider values rather than guessing a known tier.
- Inactive-token refresh uses `https://platform.claude.com/v1/oauth/token` and the public CLI OAuth
  client ID found in the official CLI and reference implementation. A successful rotated token is
  persisted immediately, even if a subsequent quota request fails. Active native tokens are not rotated.

## Claude Desktop binding

Desktop and CLI use separate sign-ins. A CLI OAuth token alone does not switch Desktop.

On macOS, Desktop's user data is `~/Library/Application Support/Claude`; standard Windows installer
profiles use `%APPDATA%\Claude`. OrbitSwitch captures only the authentication cookie rows for
`claude.ai` and authentication keys in `config.json`. It does not copy chats, projects, window settings
or the whole Electron profile. UI account-preview caches are cleared when applying a pair.

The macOS encrypted values use the installed `Claude Safe Storage` / `Claude Key` keychain entry.
Read-only validation decrypts the cookies in Rust, checks account UUID and active organization, then
queries `https://claude.ai/api/bootstrap` to confirm the server-side identity. The saved pair belongs
to one account and workspace. Session material never enters IPC, logs, screenshots or this repository.
Expired sessions or identity mismatches require another native sign-in and import.

Confirmed switching stops writers before the cookie transaction and CLI write. Cookie schema drift
aborts before deletion. Both credential stores are restored if the write/readback/commit fails;
rollback failure leaves clients closed. Desktop cookies use Chromium's existing encryption; the
additional stored snapshot is inside the system credential vault.

Windows cookie decryption supports the standard DPAPI / v10 / v11 format; app-bound v20 is rejected.
Windows Store/MSIX and custom Electron profiles, native Windows switching and all real Codex/Claude
switches remain **not verified**. Standard installation discovery must succeed before a switch;
unsupported encrypted stores are never silently reported as synchronized.
