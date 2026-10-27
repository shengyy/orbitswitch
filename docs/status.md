# Status

Platform support and feature verification scope. External client contracts live in
[antigravity-integration.md](antigravity-integration.md) and [native-integrations.md](native-integrations.md).
Update this page in the same PR whenever verification scope changes.

## Platforms

| | macOS 26+ | Windows 10/11 x64 |
|---|---|---|
| Builds and CI checks | Yes | Yes |
| Installer | Universal `.dmg` (Apple silicon and Intel), ad-hoc signed | NSIS `-setup.exe`, unsigned |
| Verified on a real installation | Yes (macOS 27) | **Not verified** ([#3](https://github.com/shengyy/orbitswitch/issues/3)) |

## Installed-client verification (macOS)

- **OrbitSwitch storage upgrade** (0.2.1 → 0.2.2, macOS 27):
  - Combined earlier and current data into 14 saved accounts across all three providers, copied
    missing Antigravity grants, retained the current grant, and consumed the old registry and vault
    entries after verification.
  - A second launch retained the identical registry and did not repeat the transfer. Official-client
    credential files and the running desktop-client processes remained unchanged.
- **Antigravity** (2.19.1, 2.22.0, 2.23.1, 2.26.0 on macOS 27):
  - OAuth client discovery, browser sign-in, adopting the active IDE sign-in, and repeated confirmed
    switches with Antigravity reopening on the target account.
  - Installed-version `User-Agent` discovery from `Resources/app.asar`, returning both Gemini and
    Claude/GPT quota groups across Pro and Ultra accounts.
  - Installed-version hub `User-Agent` on 2.24.0 with a Pro account, returning the Gemini and
    Claude/GPT groups plus a separate weekly-only Gemini 4 Argon group (read-only probe). Argon on
    Ultra accounts remains **not verified**.
  - Independent plan and quota handling: an expired Pro account updates to `Free` even when quota
    returns HTTP 403.
  - Native menus, clipboard copy and Open at Login through SMAppService.
- **Codex** (CLI 0.164.0, desktop 26.1021.31345) and **Claude** (Claude Code 2.1.310, Desktop 3.6556.2):
  - Read-only probes passed for credential discovery, user/workspace identity, plan and quota parsing
    against live endpoints.
  - Claude Desktop's captured browser session authenticated against `claude.ai/api/bootstrap` and
    matched the CLI account and active organization.
  - End-to-end Codex/Claude account switching, process shutdown/relaunch, token renewal and isolated
    browser enrollment remain **not verified**.

## Synthetic and loopback verification

- **Argon quota**: synthetic summary coverage keeps the shared weekly bucket separate from Gemini,
  preserves reset times and proto3 omitted-zero handling, and gates loopback responses on the current
  installed hub identity.
- **Account storage upgrades**: merging earlier and current registries, retaining current grants and
  desktop bindings, honoring explicit native-account removals, original Google-only metadata,
  malformed/unsupported source rejection, missing grants, failed token writes/readback, failed
  registry publication, credential-read failure, source grant rotation, interrupted cleanup, and
  repeated startup without account resurrection. Linked source/destination directories are rejected
  before any transfer or cleanup.
- **Native credential transactions**: CLI-file and SQLite cookie rollback, preserving unrelated
  settings and cookies, aborting on cookie-schema drift before deletion, distinct rollback-failure
  reporting, missing quota vs. zero, scoped limit deduplication, Codex weekly-only and dual-window
  responses, and comparable-window recommendations.
- **UI and website**: macOS Liquid Glass corner clipping and resize alignment; English and Simplified
  Chinese layouts in light and dark appearances across all three tabs, confirmation/binding/update
  sheets, and the interactive website demo.
- **App updater**: loopback signature verification, tampered payload and version-mismatch rejection,
  isolated macOS bundle replacement preserving `accounts.json`, and local signed archive verification.
  End-to-end updating of a running installed app and native Windows updates remain **not verified**.

## Known limitations

Accepted limitations are tracked under
[`deferred`](https://github.com/shengyy/orbitswitch/issues?q=is%3Aissue+is%3Aopen+label%3Adeferred).
