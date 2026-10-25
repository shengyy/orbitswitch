# Product

OrbitSwitch is a macOS menu bar and Windows tray utility for switching **your own subscription
accounts** and viewing their remaining coding quota. Visual rules live in [docs/design.md](docs/design.md),
verified scope in [docs/status.md](docs/status.md).

## Platforms and accounts

| Tab | Managed client | Quota source |
|---|---|---|
| Gemini | Antigravity desktop and `agy` CLI | Google Cloud Code model groups |
| ChatGPT | Codex desktop and CLI | Codex subscription limits |
| Claude | Claude Desktop and Claude Code | Claude subscription limits |

Each platform has its own accounts and current sign-in. Changing a tab only changes the view. Quotas
reflect the coding clients, not consumer web-chat allowances. Account identity includes provider,
user and workspace.

Add accounts through the official browser sign-in. Codex and Claude run their installed official CLI
in a temporary isolated configuration without changing the active client sign-in. **Import** captures
the current native sign-in, and new native sign-ins are discovered on refresh. Removing an account
forgets its saved credentials without signing the official client out.

Claude Desktop uses a separate browser session. Sign into the same account and workspace in both
Claude Desktop and Claude Code, then import; OrbitSwitch verifies the pair before saving it. An
unlinked account cannot be applied to an installed Claude Desktop. Custom API providers, token
environment overrides and custom desktop profiles fail before any credential change.

## Switching

Every action that closes a client shows a confirmation naming the affected app and CLI processes.
OrbitSwitch validates the target authorization first, saves the outgoing sign-in, writes and reads
back the new credentials, then reopens the installed desktop app. CLI sessions are reopened by the
user.

If writing or verifying fails, the original sign-in is restored. A failed rollback is reported
distinctly and leaves native clients closed. A launch failure after a verified write reports that
the account changed and the app must be opened manually. Operations and refreshes share a lock so
switches and token refreshes never race.

## Subscription and quota

Show the reported plan, remaining percentage, window duration and reset time, including model scopes
and nonstandard windows. Missing fields stay unavailable and are never synthesized as zero or
unlimited. Failed refreshes retain the last good numbers with a stale label.

Quotas refresh every five minutes and on panel open after a minute. Active native clients own their
token refresh; OrbitSwitch only redeems refresh tokens for inactive stored accounts. Revoked grants
require re-authentication.

The current account leads its tab. *Most left* compares fresh accounts with matching window shapes in
that tab: Claude's tightest window decides on Antigravity (Gemini breaks ties), and the primary shared
coding pool decides on Codex and Claude.

## App updates

Check stable releases at startup, daily and from the app menu. Installation requires confirmation,
verifies the signed version and package, and restarts OrbitSwitch while coding clients keep running.
Build and signing rules live in [docs/release.md](docs/release.md).

## Non-goals

No model client, API gateway, proxy, relay, shared-account pool, automatic account rotation, password
collection or self-hosted server. Supported platforms are macOS 26+ and Windows 10/11 x64.
