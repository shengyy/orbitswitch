# Product

OrbitSwitch is a small macOS menu bar / Windows tray utility for switching **your own subscription
accounts** and seeing their remaining coding quota. Visual rules live in [docs/design.md](docs/design.md),
verified behavior in [docs/status.md](docs/status.md).

## Platforms and accounts

| Tab | Managed client | Quota source |
|---|---|---|
| Gemini | Antigravity desktop and `agy` CLI | Google Cloud Code model groups |
| ChatGPT | Codex desktop and CLI | Codex subscription limits |
| Claude | Claude Desktop and Claude Code | Claude subscription limits |

Each platform has its own accounts and current sign-in. Changing a tab only changes the view. These
are coding-client limits; the ChatGPT and Gemini tabs do not claim to show their web-chat quotas.
Account identity includes the provider and workspace, not just an email address.

Add accounts through the official browser sign-in. Codex and Claude use their installed official CLI
in a temporary isolated configuration; the current client sign-in is left in place. **Import** captures
the current native sign-in. A native sign-in made outside OrbitSwitch is discovered on refresh.
Removing an account forgets its saved credentials without signing the official client out.

Claude Desktop has its own browser session. To link it, sign into the same account and workspace in
Claude Desktop and Claude Code, then import. OrbitSwitch verifies the pair before saving it. An
unlinked account cannot be applied to an installed Claude Desktop. The card provides a linking action.
Custom API providers, token environment overrides and independently configured CLI profiles do not
have the same shared sign-in contract; unsupported configurations fail before credential changes.

## Switching

Every action that closes a client shows a confirmation naming the affected app and CLI processes.
Active agent work can stop. OrbitSwitch validates the target authorization before stopping clients,
saves the latest outgoing sign-in, writes and reads back the new credentials, then opens the installed
desktop app. CLI sessions must be reopened by the user.

If a credential write or verification fails, restore the original sign-in. A failed rollback is a
separate error and leaves native clients closed. A launch failure after a verified write reports that
the account changed but the app needs to be opened manually. Operations and quota updates share a
lock so OrbitSwitch cannot refresh a token during its own switch.

## Subscription and quota

Show the reported plan, remaining percentage, each window's duration and reset time. Keep additional
model scopes and nonstandard windows. Missing information stays unavailable; it is never invented as
zero or unlimited. Temporary failures retain the last successful numbers with a visible stale label.

Refresh every five minutes and when the panel opens after a minute. Active native accounts keep the
official client's token-refresh ownership; OrbitSwitch reads its latest credential. Only inactive
stored accounts can have their refresh token redeemed by OrbitSwitch. Invalid grants require sign-in.

The current account leads its tab. *Most left* compares fresh, compatible limit windows in that tab;
it describes remaining share, not an estimate of absolute tokens. For Antigravity, Claude's tightest
window decides and Gemini breaks ties. For Codex and Claude, their main shared coding pool decides.

## App updates

Check stable releases at startup and daily, or from the app menu. Installation requires confirmation,
checks the signed version and package, and restarts OrbitSwitch. Accounts and settings survive;
coding clients keep running. The [release contract](docs/release.md) owns build and signing details.

## Non-goals

No model client, API gateway, proxy, relay, shared-account service or automatic switching when quota
runs out. No password collection or self-hosted account server. No platforms beyond macOS 26+ and
Windows 10/11 x64. Platform and integration limitations are explicit in [status](docs/status.md).
