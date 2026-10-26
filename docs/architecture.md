# Architecture

Code map, data flow, switch sequences and persistence mechanisms. External contracts live in
[antigravity-integration.md](antigravity-integration.md) and
[native-integrations.md](native-integrations.md); data inventory lives in [SECURITY.md](../SECURITY.md).

OrbitSwitch is a [Tauri 2](https://tauri.app) app: Rust executes all side effects and React renders
the backend snapshot, derives presentation state and requests operations through commands and plugins.

## Data flow

```text
 tray click ─▶ shell (popover / window) ─▶ React UI
                                            │  invoke(command)        ▲ "orbitswitch://snapshot"
                                            ▼                         │
                                  commands.rs ─▶ accounts / switcher / native / quota / scheduler / updater
                                                         │
              ┌───────────────────────────┬──────────────┴──────────────┬────────────────────────┐
              ▼                           ▼                             ▼                        ▼
   antigravity/* (install,     google/* (OAuth, loopback,    native/* (Codex/Claude   vault + registry
   live, processes, client)    Cloud Code quota & plan)      APIs, live, desktop)     (per-account state)
```

`Orbit` (`state.rs`) holds the backend state and emits a full `Snapshot` (`model.rs`) on every update.
The frontend never holds tokens or cookies; it keeps dialog state and derives account ordering, quota
severity and recommendations from the snapshot.

## Backend modules (`src-tauri/src`)

| Module | Owns |
|---|---|
| `main.rs` / `lib.rs` | Entry point, plugin wiring, startup and command registration |
| `commands.rs` | Tauri command handlers |
| `state.rs` | Shared `Orbit` state, snapshot emission, operation lock, token minting |
| `model.rs` | Frontend snapshot types (`AccountView`, `ProviderStatus`, `QuotaState`, `Operation`) |
| `provider.rs` | Provider identities (`Antigravity`, `Codex`, `Claude`) |
| `accounts.rs` | Antigravity browser sign-in, removal and live sign-in adoption |
| `switcher.rs` | Antigravity account switch, stop and restart |
| `native/` | Codex and Claude identity/quota APIs, live credential stores, Claude Desktop pairing, process control and rollback transactions |
| `quota.rs` | Concurrent quota and plan refresh |
| `scheduler.rs` | 5-minute background upkeep and scheduled update checks |
| `updater.rs` / `updater_tests.rs` | Signed app update checks, installation and loopback tests |
| `autostart.rs` | Open at Login (SMAppService on macOS, Run key on Windows) |
| `registry.rs` | Account metadata and fingerprints in `accounts.json` |
| `registry_migration.rs` / `registry_migration_tests.rs` | One-time storage transfer before startup, copy journal, destination verification and resumable source cleanup |
| `vault.rs` | Antigravity refresh tokens in the OS credential store |
| `tokens.rs` | In-memory access-token cache |
| `secret_store/` | macOS Keychain (`/usr/bin/security`) and Windows Credential Manager primitives |
| `antigravity/` | Antigravity installation, `app.asar` version reader, OAuth client discovery, credential bundle, live store and processes |
| `google/` | Google OAuth (PKCE), loopback callback server and Cloud Code client |
| `shell.rs` / `tray.rs` | Popover / tray window and tray icon |
| `error.rs` | Serializable error codes localized by the frontend |
| `system_tests.rs` | Opt-in read-only checks against installed Antigravity (`--ignored`) |

## Frontend (`src`)

| Path | Owns |
|---|---|
| `main.tsx` | React root and shared stylesheets |
| `types.ts` | TypeScript mirror of `model.rs` |
| `ipc.ts` | Command and event names |
| `app.tsx` / `app.css` | Panel composition, dialog routing and account ordering |
| `menus.ts` | Native context menus and clipboard / URL actions |
| `quota.ts` | Quota severity, usable share and *Most left* recommendation |
| `providers.ts` | Provider tab and client labels |
| `i18n.ts` / `format.ts` | English / Simplified Chinese copy and time / percentage formatting |
| `hooks/` | Snapshot subscription, macOS popover auto-sizing and relative-time ticker |
| `components/` | UI components, icons and styles (`sheet.css` shared across sheets) |
| `styles/` | Design tokens and base styles |

## Operations and concurrency

One user operation runs at a time (`Orbit::begin` returns `Busy` otherwise): `adding`, `importing`,
`removing`, `switching` (with step progress), `stopping`, `restarting` or `updating`. A shared async
`sync_lock` serializes live-credential sync, quota/token refresh and switches so a background refresh
cannot race an active switch. Client installation and process checks run outside the snapshot mutex.

### Antigravity switch sequence

1. **Preparing**: redeem the target account's refresh token; stop immediately on `invalid_grant` and
   mark the account `Reauth`.
2. Save the outgoing live refresh token back to `vault.rs`.
3. **Closing**: stop Antigravity, its helpers, language server and `agy` CLI (`SIGTERM` / `taskkill`,
   then force-kill after 8 s).
4. **Writing**: write keychain and `~/.gemini/jetski-standalone-oauth-token`, read back and verify;
   restore the previous credential and relaunch on failure.
5. **Launching**: open Antigravity and wait for its main process.

### Native switch sequence (Codex / Claude)

1. **Preparing**: verify client configuration, re-read the live CLI credential, refresh the target
   token only if it is not the active native account, and verify target profile, usage identity and
   any paired Claude Desktop session.
2. **Closing**: send `SIGTERM` / `taskkill` to recognized client processes and wait up to 8 s without
   force-killing.
3. Capture outgoing CLI and Claude Desktop state for rollback and save updated outgoing credentials.
4. **Writing**: `native/transaction.rs` writes CLI credentials (and paired Claude Desktop cookies /
   config), reads back and verifies identity, and updates the native fingerprint. On failure, both
   CLI and Desktop stores roll back; failed restoration returns `RollbackFailed` and leaves clients
   closed.
5. **Launching**: reopen the installed desktop app.

### App updates

`updater.rs` holds the pending Tauri `Update` and check state inside `Orbit`; snapshots expose only
version, release notes and progress. Confirmed installation downloads and verifies the signed package
before replacing the bundle, then restarts OrbitSwitch (`app.restart()` on macOS, NSIS on Windows).

## Persistence

- Before loading normal account state, `registry_migration.rs` consumes the previous storage
  namespace into the current one. Current metadata, grants and explicit native-account removals take
  precedence. A journal records hashes for staged credential copies before writes, allowing retries
  to repair an incomplete copy without treating it as an existing current grant. The merged registry
  is atomically written, synced and read back before source cleanup. Cleanup interruptions resume on
  the next launch. Codex / Claude session storage is already shared and is left intact. No old-store
  fallback remains after the transfer; official client credentials and processes are unaffected.
- `registry.rs` atomically writes `accounts.json` via a temporary file rename. Refresh-token and
  native-credential fingerprints prevent re-adopting a removed account until the native client signs
  into a different session or the user explicitly imports it.
- `vault.rs` stores Antigravity refresh tokens under service `orbitswitch`. On macOS,
  `secret_store/macos.rs` uses `/usr/bin/security` so items share the system tool's ACL with
  Antigravity's `go-keyring`.
- `native/storage.rs` splits Codex and Claude session bundles into bounded chunks under service
  `orbitswitch.sessions` and publishes a new generation index only after all chunks succeed.
