# Native subscription integrations

Credential, identity, desktop-binding and quota contracts for Codex and Claude. Antigravity contracts
live in [antigravity-integration.md](antigravity-integration.md); verification scope lives in
[status.md](status.md).

## References

| Source | Contract |
|---|---|
| [OpenAI auth storage](https://github.com/openai/codex/blob/main/codex-rs/login/src/auth/storage.rs) | `CODEX_HOME`, `file` / `keyring` / `auto` storage modes, canonical-path SHA-256 key hashing |
| [codex-auth](https://github.com/Loongphy/codex-auth) | Native `auth.json` preservation and `wham/usage` endpoint |
| [codex-account-switcher](https://github.com/liuzhao1225/codex-account-switcher) | Desktop shutdown, outgoing credential save, write/verify and rollback |
| [claude-swap](https://github.com/realiti4/claude-swap) | CLI keychain/file credentials, `.claude.json` `oauthAccount` and active-client refresh ownership |
| [CodexBar](https://github.com/steipete/CodexBar) | Usage/profile schemas, window durations, model scopes and stale-data handling |
| [Claude-Code-Desktop-Switcher](https://github.com/PriyanshuGeTRekT/Claude-Code-Desktop-Switcher) | Independent Electron session storage in Claude Desktop |

Read-only macOS probes verified Codex CLI 0.164.0, Codex desktop 26.1021.31345 (`com.openai.codex`),
Claude Code 2.1.310 and Claude Desktop 3.6556.2 (`com.anthropic.claudefordesktop`).

## Codex

- **Credentials**: `~/.codex/auth.json` by default (`CODEX_HOME` overrides). `config.toml` sets
  `cli_auth_credentials_store` (`file`, `keyring` or `auto`); unknown modes or
  `features.secret_auth_storage = true` fail closed. Custom `CODEX_HOME` is rejected when switching
  an installed desktop app.
- **Keyring**: service `Codex Auth`, account `cli|<first 16 hex chars of SHA-256(canonical_root)>`,
  accessed through the native `keyring` crate on macOS and Windows.
- **Identity**: `chatgpt_user_id` (or `sub`) from `tokens.id_token` claims plus `tokens.account_id`
  for the workspace.
- **Usage and plan**: `GET https://chatgpt.com/backend-api/wham/usage` with bearer token and
  `ChatGPT-Account-Id`. Returned `user_id` and `account_id` must match the credential. Windows are
  classified by `limit_window_seconds` (`18000` → 5h, `604800` → weekly, others preserved in `extra`),
  with `remaining = 1 - used_percent / 100`. Weekly-only responses render only a weekly meter.
- **Token renewal**: inactive accounts refresh via `POST https://auth.openai.com/oauth/token` with
  public client `app_EMoamEEZ73f0CkXaXp7hrann`.

## Claude Code

- **Credentials**: macOS reads keychain service `Claude Code-credentials` (account `$USER`) first and
  falls back to `~/.claude/.credentials.json`; Windows uses the file. `CLAUDE_CONFIG_DIR` changes the
  config root; `CLAUDE_SECURESTORAGE_CONFIG_DIR` (or `CLAUDE_CONFIG_DIR`) appends
  `-<first 8 hex chars of SHA-256(NFC(path))>` to the keychain service name.
- **Write scope**: updates only `claudeAiOauth` in `.credentials.json` / keychain and `oauthAccount`
  in `~/.claude.json` (or `<CLAUDE_CONFIG_DIR>/.claude.json`), preserving all other settings.
- **Profile and usage**: `GET https://api.anthropic.com/api/oauth/profile` and `/usage` with bearer
  token and `anthropic-beta: oauth-2025-04-20`. `remaining = 1 - utilization / 100`. Active
  `weekly_scoped` limits take precedence over legacy `seven_day_sonnet` / `seven_day_opus` fields.
- **Token renewal**: inactive accounts refresh via `POST https://platform.claude.com/v1/oauth/token`
  with public client `9d1c250a-e61b-44d9-88ed-5944d1962f5e` and persist rotated refresh tokens
  immediately.

## Claude Desktop binding

Claude Desktop uses a separate Electron session in `~/Library/Application Support/Claude` (macOS) or
`%APPDATA%\Claude` (Windows).

- **Captured state**: authentication cookies for `claude.ai` / `.claude.ai` (`sessionKey*`,
  `lastActiveOrg`, `cf_clearance`, `__cf_bm`) from `Network/Cookies` or `Cookies`, plus
  `lastKnownAccountUuid` and `oauth:tokenCache*` from `config.json`.
- **Verification**: decrypts cookies in Rust (`v10` via keychain `Claude Safe Storage` / `Claude Key`
  on macOS; DPAPI / `v10` / `v11` via `Local State` on Windows; `v20` app-bound encryption is
  rejected), checks `lastKnownAccountUuid` and `lastActiveOrg`, and verifies server-side identity via
  `GET https://claude.ai/api/bootstrap`.
- **Switch transaction**: stops running client writers first, aborts on SQLite cookie-schema drift
  before deleting rows, clears `bootSideSessions` and `bootSidebarPlaceholder` UI caches, verifies
  readback, and restores both CLI and Desktop state if any step fails.
