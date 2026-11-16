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
  - Independent plan and quota handling: an expired Pro account updates to `Free` even when quota
    returns HTTP 403.
  - Native menus, clipboard copy and Open at Login through SMAppService.
  - The original OAuth request reaches Google's sign-in form in a fresh Chrome 156.0.8078.48
    incognito context on macOS 27.0.1. Completing private-window account enrollment remains
    **not verified**.
- **Codex** (CLI 0.164.0 / 0.169.0, desktop 26.1021.31345) and **Claude** (Claude Code 2.1.310 / 2.1.325, Desktop 3.6556.2):
  - Read-only probes passed for credential discovery, user/workspace identity, plan and quota parsing
    against live endpoints.
  - Under a minimal macOS GUI environment (`PATH=/usr/bin:/bin:/usr/sbin:/sbin`, macOS 27.0.1), CLI
    discovery resolved `codex` 0.169.0, `claude` 2.1.325 and Homebrew `node` v26.12.1 through the
    exported runtime `PATH`.
  - Claude Desktop's captured browser session authenticated against `claude.ai/api/bootstrap` and
    matched the CLI account and active organization.
  - End-to-end Codex/Claude account switching, process shutdown/relaunch, token renewal and isolated
    browser enrollment remain **not verified**.
- **Codex local-project recovery** (desktop 26.1103.41825, macOS 27.0.1):
  - Read-only inspection verified account-scoped local-project and chat-membership storage in the
    installed desktop. The recovery planner successfully combined saved-account projects and
    memberships in memory without writing native state or credentials or stopping processes.
  - An authorized client stop/relaunch using the native Rust recovery path restored local projects
    and chat memberships. Native-state readback and the desktop's project-list API confirmed the
    recovered projects; credential-file bytes were unchanged.
  - Account-to-account switching and the OrbitSwitch **Restart** button remain **not verified**.
- **Codex sidebar contract** (desktop 26.1103.60933, macOS 26.3.1):
  - Read-only native-state and installed-source inspection confirmed that account-specific collapse,
    full-list visibility and project-order atoms can hide projects whose data is already present.
  - The new forced expansion and workspace-layout preservation remain **not verified** end to end
    on a running desktop; their storage contract and synthetic verification are described below.

## Synthetic and loopback verification

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
- **Native CLI runtime resolution**: preserving symlinked launcher paths while matching canonical
  executables, numeric semver ordering for versioned Node managers, executing `#!/usr/bin/env node`
  launchers when the GUI `PATH` omits the Node directory, bounded `stderr` draining, cancellation, and
  redacted `cli_runtime` vs. `oauth_denied` classification.
- **Codex local projects**: primary-directory matching, retaining destination project IDs/settings
  and explicit memberships, distinct projects with overlapping secondary roots, saved-account scope,
  cloud/remote exclusion, A → B → A with new chats, idempotent repair, unsupported-state rejection,
  atomic state/backup readback, concurrent-writer rejection and raw-file rollback after project or
  later credential-transaction failures.
- **Codex local sidebar**: outgoing local order and per-project expansion through primary-directory
  aliases, preserving destination private/cloud state, forced project-section/full-list expansion
  on every switch/restart, repairing an already migrated but shortened sidebar, explicit outgoing
  ownership during signed-out startup, empty-project no-op and unsupported-shape rejection.
- **UI and website**: macOS Liquid Glass corner clipping and resize alignment; English and Simplified
  Chinese layouts in light and dark appearances across all three tabs, confirmation/binding/update
  sheets, and the interactive website demo.
- **Gemini browser enrollment**: the original authorization URL preserves callback and state;
  loopback callbacks require matching state and no browser cookies. The waiting sheet's copy action
  has preparation, success and retry states. Real private-window Google sign-in remains **not verified**.
- **App updater**: loopback signature verification, tampered payload and version-mismatch rejection,
  isolated macOS bundle replacement preserving `accounts.json`, and local signed archive verification.
  End-to-end updating of a running installed app and native Windows updates remain **not verified**.

## Known limitations

Accepted limitations are tracked under
[`deferred`](https://github.com/shengyy/orbitswitch/issues?q=is%3Aissue+is%3Aopen+label%3Adeferred).
