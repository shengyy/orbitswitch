# Changelog

All notable changes to OrbitSwitch are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.2.2] - 2026-10-26

### Fixed

- Recover saved accounts after the 0.2.1 storage-identifier change. On first launch, merge the
  previous account registry into OrbitSwitch's current store and transfer saved Antigravity
  authorizations without replacing grants already saved by 0.2.1. Codex / Claude sessions and
  Claude Desktop bindings are retained. No account re-enrollment is needed for intact saved grants.
- Make the transfer resumable: persist copy fingerprints before credential writes, verify the
  destination before removing the source, and retain recoverable state on failure. Only the current
  OrbitSwitch store is used after completion; native client sign-ins and processes are untouched.
- Reject linked source/destination directories before transfer so a directory-link workaround cannot
  cause source cleanup to remove the destination. Remove such a link before upgrading.

## [0.2.1] - 2026-10-25

### Changed

- Use `io.github.shengyy.orbitswitch` as the bundle identifier, `orbitswitch` as the Google
  credential-store service, and `OrbitSwitch` as the Windows login-item name.

### Upgrade

- This version changed the account storage namespace without transferring existing data. Upgrade to
  0.2.2 or later to recover the saved accounts. Keep the earlier account data and credential-store
  entries until that upgrade finishes. Unrelated old logs, caches and Windows Run entries can be
  removed manually after closing any older app instance.

## [0.2.0] - 2026-10-25

### Added

- Gemini / ChatGPT / Claude tabs with independent accounts and current-client state.
- Codex and Claude subscription sign-in, current-account import, plan/quota adapters and confirmed
  switching. Claude Desktop pairs with Claude Code by verified account and workspace identity.
- Native credential rollback and identity checks, stale-quota labels, model scopes and additional
  limit durations. Active clients retain ownership of their token refresh.
- Codex quota windows follow the provider response, including weekly-only accounts. Only comparable
  windows participate in account recommendations; the website demo follows the same rule.
- Bilingual website with an interactive fictional three-platform demo and release download resolver.

### Changed

- Three-segment rotary selector icon across the app, tray, website and installers.

### Manual cleanup

- If the app crashes during an isolated Codex/Claude login, close that login process and remove its
  `orbitswitch-login-*` directory in the OS temporary folder (plus the matching temporary
  `Claude Code-credentials-<hash>` keychain entry on macOS, if created).

### Verification scope

- macOS and Windows CI passed. Installed-client identity and quota reads, including Claude Desktop
  binding, were verified on macOS. Real Codex/Claude switching, browser enrollment, token renewal
  and native Windows behavior remain **not verified**. See [docs/status.md](docs/status.md).

## [0.1.8] - 2026-10-16

### Fixed

- Restore missing Gemini quotas by sending the installed Antigravity version in Cloud Code requests
  and re-reading it on each refresh.

## [0.1.7] - 2026-10-13

### Fixed

- Refresh the subscription badge even when the quota request fails, so an expired subscription
  updates from Pro to Free.

## [0.1.6] - 2026-10-06

### Fixed

- Clip the macOS popover content inside its native Liquid Glass container.

### Changed

- Highlight the whole account card on hover or keyboard focus and move `⋯` to the top-right row.

## [0.1.5] - 2026-10-06

### Fixed

- Disable the extra macOS window shadow outside the Liquid Glass corners.

## [0.1.4] - 2026-10-04

### Added

- Signed app updates checked at startup, daily and from the app menu.

## [0.1.3] - 2026-10-04

### Fixed

- Prevent account tags and action controls from overlapping on account cards.

## [0.1.2] - 2026-10-03

### Fixed

- Retain native menu item resources so click handlers stay active in Tauri 2.12.
- Copy Email writes through the system clipboard plugin.
- Ad-hoc sign macOS bundles with a stable bundle identifier so Open at Login survives updates.

### Added

- Website link in the `⋯` menu.

## [0.1.1] - 2026-10-03

### Changed

- Register Open at Login through SMAppService on macOS.
- Responsive website layout and higher-density screenshots.

## [0.1.0] - 2026-10-03

### Added

- macOS 26+ Liquid Glass menu bar popover and Windows tray window.
- Quota meters, reset times and *Most left* recommendation.
- Browser OAuth sign-in with PKCE and automatic adoption of the active Antigravity account.
- Confirmed switching with process shutdown, credential verification and rollback on failure.
- Restart and quit controls, Open at Login, logs, and English / Simplified Chinese UI.
