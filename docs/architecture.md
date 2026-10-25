# Architecture

This page owns OrbitSwitch's code map, data flow, switch sequence and persistence mechanisms. External
contracts and verified versions live in [antigravity-integration.md](antigravity-integration.md) and
[native-integrations.md](native-integrations.md); data
inventory and storage locations live in [SECURITY.md](../SECURITY.md).

OrbitSwitch is a [Tauri 2](https://tauri.app) app: Rust executes side effects and React renders the backend
snapshot, derives presentation data and requests operations through commands and Tauri plugins.

## Data flow

```text
 tray click ─▶ shell (popover / window) ─▶ React UI
                                            │  invoke(command)        ▲ "orbitswitch://snapshot"
                                            ▼                         │
                                  commands.rs ─▶ accounts / switcher / quota / scheduler / updater
                                                         │
                     ┌───────────────────────────────────┼─────────────────────────────┐
                     ▼                                   ▼                             ▼
          antigravity/* (install, live          google/* (OAuth, loopback,     vault + registry
          credential, processes, client)        Cloud Code quota)              (per-account state)
```

The backend keeps one `Orbit` state (`state.rs`). `Orbit::update` publishes state changes as a full
`Snapshot` (`model.rs`). The frontend never holds tokens. It keeps dialog state and derives account
ordering, quota severity and the recommendation from the snapshot. Account, credential and process
operations run in Rust; native menus, clipboard writes and URL opening use Tauri's APIs and plugins.

## Backend modules (`src-tauri/src`)

| Module | Owns |
|---|---|
| `main.rs` | Executable entry point that calls `lib.rs` |
| `lib.rs` | Plugin wiring, startup, command registration |
| `commands.rs` | App commands: delegates account and process behavior, handles app / shell actions |
| `state.rs` | Shared state, snapshot emission, the single operation slot, token minting |
| `model.rs` | Types sent to the frontend, including provider-neutral plans and quota windows |
| `provider.rs` | Provider identities |
| `native/` | Codex/Claude identity and quota APIs, native credential stores, desktop pairing and rollback |
| `accounts.rs` | Browser sign-in, removal, adopting a sign-in made inside Antigravity |
| `switcher.rs` | Switching accounts, stopping and restarting Antigravity |
| `quota.rs` | Refreshing quota and plan for each account |
| `scheduler.rs` | The 5-minute background pass, including update checks when due |
| `updater.rs` / `updater_tests.rs` | Signed app update checks, installation and loopback verification |
| `autostart.rs` | Open at Login: SMAppService on macOS, the per-user Run key on Windows |
| `registry.rs` | Account metadata in `accounts.json` (no secrets) |
| `vault.rs` | Per-account refresh tokens in the credential store |
| `tokens.rs` | In-memory access tokens |
| `secret_store/` | macOS Keychain / Windows Credential Manager primitives |
| `antigravity/` | Install location and version, OAuth client discovery, credential format, live credential, processes |
| `google/` | OAuth (PKCE), loopback callback server, Cloud Code endpoints, User-Agent |
| `shell.rs` / `tray.rs` | The popover or window, and the tray icon |
| `error.rs` | Error codes the frontend localizes |
| `system_tests.rs` | Read-only checks against a real installation (`--ignored`) |

## Frontend (`src`)

| Path | Owns |
|---|---|
| `main.tsx` | React mounting and shared stylesheet loading |
| `types.ts` | Mirror of `model.rs` |
| `ipc.ts` | Command and event names |
| `app.tsx` / `app.css` | Surface composition, dialog state, account ordering and panel layout |
| `menus.ts` | Native context menus and their Tauri API / plugin actions |
| `quota.ts` | Presentation derived from snapshot quotas: severity, usable share and recommended account |
| `providers.ts` | Presentation labels for provider tabs and affected clients |
| `i18n.ts` / `format.ts` | Strings (English, Simplified Chinese) and time / percent formatting |
| `hooks/` | Snapshot subscription, popover auto-size, relative-time ticker |
| `components/` | UI components, shared icons and colocated styles; sheets share `sheet.css` |
| `styles/` | Design tokens and base styles |

## Antigravity adapters

### Installed version and request identity

`antigravity/version.rs` reads the root `package.json` from the installed Electron `app.asar`, using
bounded header and entry reads. `Install::version` exposes it without launching the app. Each
quota/plan refresh creates a `google/cloudcode.rs` client with the current installed version and
overrides the shared HTTP client's generic OrbitSwitch identity on both requests. Unreadable metadata
fails the lookup; the existing refresh error handling retains the last good quota and plan.
The external identity contract is in [Quota and plan](antigravity-integration.md#quota-and-plan).

### Credential access

`antigravity/live.rs` reads the keyring first and writes both the keyring and fallback file, matching
the [external storage contract](antigravity-integration.md#signed-in-credential).

`secret_store/macos.rs` invokes `/usr/bin/security`, as Antigravity's go-keyring does, so each item's ACL
stays bound to that system tool rather than OrbitSwitch's per-build signature. This avoids keychain prompts
for OrbitSwitch and Antigravity.

`antigravity/credential.rs` preserves unknown fields when decoding and re-encoding an existing bundle.
A switch creates a fresh bundle from Google's token grant, keeping the previous `auth_method` or using
`consumer` when there was none.

### OAuth client discovery

`antigravity/oauth_client.rs` scans the installed Antigravity's `bin/` directory for secret candidates,
cuts each to 35 characters because Go packs literals back to back, and probes Google's token endpoint
with a bogus authorization code. It selects the candidate returning `invalid_grant`; the client and
response contract are in [OAuth client](antigravity-integration.md#oauth-client).

### Process control

`antigravity/process.rs` sends SIGTERM (Windows: `taskkill` without `/F`) to the app and CLI, waits up to
8 s, then kills whatever is left. It relaunches with `open <bundle>` (Windows: the executable, detached)
and waits for the main process. The processes that must exit and why are in
[Processes](antigravity-integration.md#processes).

## Operations

One operation runs at a time (`Orbit::begin` returns `Busy` otherwise): `adding`, `switching` (with its
current step), `stopping`, `restarting`, `updating` (download, verification, installation, restart). The background pass skips while one is running. A shared async lock also serializes credential synchronization, quota/token refresh and switches; a native refresh cannot race an OrbitSwitch switch. Process/installation probes are cached outside the snapshot state mutex.

### Antigravity switch sequence

1. **Preparing**: refresh the target's token. If Google refuses (`invalid_grant`), stop here; nothing has
   been touched, and the account is marked for re-authorization.
2. Save the current sign-in back to its account's vault entry (Antigravity may hold a newer token).
3. **Closing**: stop every Antigravity process.
4. **Writing**: write keychain + fallback file, read back and verify. On failure restore the previous
   credential (or clear it if there was none) and relaunch.
5. **Launching**: open Antigravity and wait for its main process.

### App updates

`updater.rs` owns the pending Tauri `Update`, check timing and check reservation inside `Orbit`.
Snapshots expose only the version, notes and check state; the frontend never receives an installer
URL, signature or updater resource. Read-only checks run alongside quota upkeep when due.
Installation claims `Orbit::begin`, rejects an in-flight check or a changed confirmed version, and
keeps the slot until failure or restart. Tauri downloads and verifies before installing; errors keep
the offer available for retry. macOS explicitly restarts OrbitSwitch after installation; the Windows
NSIS updater handles exit and relaunch. The release contract is in [release.md](release.md).

### Adopting a sign-in made in the IDE

`accounts.json` keeps a fingerprint (truncated SHA-256) of the last refresh token seen in Antigravity's
credential. A different fingerprint means a new sign-in: the account is adopted once. A removed account
therefore stays removed until Antigravity signs in again.

## Persistence

`registry.rs` writes metadata to a temporary JSON file and renames it over `accounts.json`.
`vault.rs` delegates refresh-token storage to `secret_store/`; `tokens.rs` caches access tokens in memory.
The data inventory, storage locations and logging boundary are owned by
[SECURITY.md](../SECURITY.md#what-is-stored).

## Native adapters

Codex and Claude use `native/api.rs` to normalize their official subscription responses into the
same `model.rs` plan/group/window types as Antigravity. Additional durations and scoped pools remain
visible; missing limits are not replaced with zero. Failed refreshes mark retained data stale.

`native/live.rs` resolves the official CLI's native store and preserves unrelated settings. Isolated
browser enrollment runs the official CLI under a temporary private configuration. `native/desktop.rs`
verifies and stores the separate Claude desktop sign-in by account and workspace. Credentials stay
inside the backend; snapshots expose binding/alignment status only. An unconfirmed Desktop/CLI pair
has no shared Current tag, while the CLI identity still owns refresh-token protection internally.

The native switch verifies the target, stops recognized client writers without forced termination,
captures the latest outgoing state, writes the CLI and paired desktop credentials, reads them back,
commits the fingerprint and reopens the desktop app. `native/transaction.rs` runs both rollback paths
on failure and reports failed restoration distinctly. Tests use isolated JSON and SQLite fixtures.

`native/storage.rs` stores session bundles in bounded OS-vault chunks. New chunks are complete before
publishing a generation index; failure preserves the previous index. Native account keys include
provider, user and workspace. An ignored native identity prevents token rotation from resurrecting a
removed account. Explicit import or a different subsequent sign-in enables discovery again.
