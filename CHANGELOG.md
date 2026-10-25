# Changelog

All notable changes to OrbitSwitch are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.2.0] - 2026-10-25

### Added

- Gemini / ChatGPT / Claude tabs with independent accounts and current-client state.
- Codex and Claude subscription sign-in, current-account import, plan/quota adapters and confirmed
  switching. Claude Desktop is paired with Claude Code by verified account and workspace identity.
- Native credential rollback and identity checks, stale-quota labels, model scopes and additional
  limit durations. Active clients retain ownership of their token refresh.
- A redesigned bilingual website with a fictional interactive three-platform demo and accurate
  release download status.

### Changed

- Replace the Antigravity-themed planet with a three-segment rotary selector logo across the app,
  tray, website and installers.
- Rename the project to OrbitSwitch and the repository to `shengyy/orbitswitch`. Preserve the
  `io.github.shengyy.agyorbit` application identifier, existing Google credential service and saved
  account directory. No data migration is required.

### Manual cleanup

- If installing a renamed `.app` alongside an existing `AgyOrbit.app`, quit the old app and remove
  only that old application bundle; keep its application-data and keychain entries. Avoid running
  both names together. Existing in-place updates can retain the original bundle directory name.
- If the app crashes during an isolated Codex/Claude login, after closing that login process remove
  only its `orbitswitch-login-*` directory in the OS temporary folder. On macOS, remove the associated
  temporary `Claude Code-credentials-<hash>` keychain entry if one was created. Do not remove the
  unsuffixed native credential or unrelated profile entries.

### Verification scope

- macOS and Windows CI passed. Installed-client identity and quota reads, including Claude Desktop
  binding, were verified on macOS. Real Codex/Claude switching, browser enrollment, token renewal
  and native Windows behavior remain **not verified**. See the complete
  [verification scope](https://github.com/shengyy/orbitswitch/blob/v0.2.0/docs/status.md).

## [0.1.8] - 2026-10-16

### Fixed

- Restore missing Gemini quotas by sending the installed Antigravity version in Cloud Code requests.
  Read the version again on refresh so Antigravity updates do not leave a stale request identity.

## [0.1.7] - 2026-10-13

### Fixed

- Refresh the subscription badge even when the quota request fails, so an expired subscription can
  change from Pro to Free instead of keeping the previously saved plan.

## [0.1.6] - 2026-10-06

### Fixed

- Clip the macOS popover content inside its native Liquid Glass container, so page backgrounds and
  confirmation backdrops cannot draw rectangular corners outside the glass surface.

### Changed

- Remove the redundant Switch pill from account cards. Highlight the whole card on hover or keyboard
  focus, retain keyboard activation and move More to the far right of the name and tags row.

## [0.1.5] - 2026-10-06

### Fixed

- Disabled the extra macOS window shadow that drew a dark outline outside the Liquid Glass corners.

## [0.1.4] - 2026-10-04

### Added

- App updates: check at startup and daily, or from the app menu. View the release notes and confirm
  before downloading a signed update and restarting AgyOrbit; Antigravity keeps running.
- GitHub releases include signed updater packages and the update manifest for macOS and Windows.
  Install this first updater-enabled release manually once; older versions cannot update themselves.

## [0.1.3] - 2026-10-04

### Fixed

- The Switch button no longer overlaps the Most left tag. Account tags follow the name and plan badge,
  and the button has its own space beside the email.

## [0.1.2] - 2026-10-03

### Fixed

- Menu items did nothing when clicked (Show Logs, GitHub, Quit, and the account menu's Sign In Again,
  Copy Email and Remove). Tauri 2.12 drops items declared inline in `Menu.new` right after building the
  menu, which removes their click handlers; every item is now created as its own resource.
- Copy Email writes through the system clipboard; the web clipboard API refuses writes that do not come
  from a click inside the page.
- Open at Login survives updates on macOS. Builds are now ad-hoc signed with the bundle identifier, so
  every version has the same code identity, which is what macOS ties the login item to. Turn it on once
  more after updating from 0.1.1.

### Added

- *AgyOrbit Website* in the `⋯` menu.

## [0.1.1] - 2026-10-03

### Changed

- Open at Login registers AgyOrbit as a login item through SMAppService on macOS, so System Settings
  lists it by name and icon instead of as a background item from an unidentified developer. If you
  turned it on in 0.1.0, delete `~/Library/LaunchAgents/AgyOrbit.plist` and turn it on again. Windows
  keeps the per-user Run key.
- Website: the page scales with large displays instead of leaving wide margins, with sharper
  screenshots.

## [0.1.0] - 2026-10-03

### Added

- Menu bar popover on macOS 26+ (Liquid Glass) and a tray window on Windows.
- Every account's Gemini and Claude quota, 5-hour and weekly windows, with reset times and a
  "most left" hint for the account worth switching to.
- Browser sign-in (OAuth with PKCE on a loopback port) to add or re-authorize accounts without signing
  out of Antigravity; the account Antigravity is already signed in with is adopted automatically.
- Confirmed one-click switching: stops Antigravity and the `agy` CLI, writes the credential to the
  keychain and its fallback file, verifies it, relaunches, and restores the previous sign-in on failure.
- Restart and quit Antigravity, open at login, logs.
- English and Simplified Chinese UI.
- Website at https://shengyy.github.io/agyorbit/ with download links to the latest release.

### Verified

- macOS 27 with Antigravity 2.19.1: OAuth client discovery, browser sign-in, quota and plan, and a
  switch between two Google AI Pro accounts (credential written, verified, Antigravity relaunched).
  Windows support compiles in CI but has not been verified on a real installation.
