# Status

This page owns platform support and the scope of feature verification. Antigravity's external
contracts and their verified versions live in [antigravity-integration.md](antigravity-integration.md).
The version itself lives in `src-tauri/Cargo.toml`, published
builds on [Releases](https://github.com/shengyy/orbitswitch/releases), and the change history in
[CHANGELOG.md](../CHANGELOG.md). Update this page in the same PR as anything that changes what has been
verified.

## Platforms

| | macOS 26+ | Windows 10/11 x64 |
|---|---|---|
| Builds and CI checks | Yes | Yes |
| Installer | Universal `.dmg` (Apple silicon and Intel), ad-hoc signed | NSIS `-setup.exe`, unsigned |
| Verified on a real installation | Yes: macOS 27, Antigravity 2.19.1 | **Not verified** ([#3](https://github.com/shengyy/orbitswitch/issues/3)) |

## Multi-platform adapters (Unreleased)

- Read-only probes on macOS passed against Antigravity 2.26.0, Codex CLI 0.164.0 / desktop
  26.1021.31345, and Claude Code 2.1.310 / Desktop 3.6556.2.
- Codex credential discovery, returned user/workspace identity, plan and quota parsing passed against
  the live usage endpoint. Claude profile/usage reads passed; Desktop's captured browser session
  authenticated successfully and matched the CLI account and active organization.
- Real Codex/Claude account switching, process shutdown/relaunch, token renewal and browser account
  enrollment are **not verified**. No installed client's credential was rewritten for these probes.
- Synthetic tests cover native CLI-file rollback, preserving unrelated settings and cookies, rejecting
  cookie-schema drift before deletion, failed rollback reporting, missing quota vs. zero and scoped
  limit deduplication. These tests do not replace a real client switch.
- Browser checks with fictional accounts passed for all three tabs in both languages and appearances,
  including confirmation routing, stale/empty/binding states and narrow layouts. The website passed
  desktop and mobile layout, language and interactive-demo checks.
- Native Windows behavior is **not verified**. Standard installer paths are detected; Store/MSIX,
  independently configured profiles and app-bound cookie encryption are unsupported. CI compilation
  does not establish real account switching on Windows.
- The first multi-platform release is not published. Website downloads identify 0.1.x as the legacy
  Antigravity-only build rather than presenting it as the new three-tab product.

## Verified on macOS (0.1.2)

- OAuth client discovery from the installed Antigravity, browser sign-in, adopting the IDE's own
  sign-in.
- Quota and plan for seven Google AI Pro accounts.
- Repeated switches between accounts, with Antigravity reopening on the new account.
- Every menu item, Open at Login through SMAppService, and the login item surviving an update.

Each external fact these rely on, and the Antigravity version it was checked against, is listed in
[antigravity-integration.md](antigravity-integration.md).

## Subscription refresh verification

- Read-only checks on macOS 27 with Antigravity 2.22.0: an expired Pro account's plan lookup reports
  Free while its quota lookup returns 403; an active Ultra account still reports Ultra. Neither
  response supplies a subscription expiry or renewal date. See the
  [API contract](antigravity-integration.md#quota-and-plan).
- Backend regression tests cover persisting the new Free plan despite quota failure, preserving
  previous quota data on failure, independent plan failures, revoked authorization and account removal
  during a refresh. The updated behavior in a running installed OrbitSwitch is **not verified**.

## Quota request identity verification

- Read-only checks on macOS 27 with Antigravity 2.23.1 reproduced HTTP 200 responses missing Gemini
  with the unversioned product identity. The installed-version identity returned Gemini and
  Claude/GPT for six Pro accounts and one Ultra account. The active account's groups and weekly
  values matched the official app's freshly fetched summary. No separate Gemini 4 Argon group was
  returned; see the [API contract](antigravity-integration.md#quota-and-plan).
- Regression tests cover reading the installed version, invalid package metadata, the HTTP identity
  on both quota and plan requests, and an Antigravity upgrade between refreshes. The real installation
  system test now rejects a successful summary that omits Gemini. The fix in a running installed
  OrbitSwitch and native Windows version discovery are **not verified**.

## Verified with synthetic accounts

- Native Tauri popover on macOS 27 with the production window code: a solid red page background is
  clipped at all four corners by the Liquid Glass content container. The account list and switch
  confirmation are rounded in light and dark appearance; opening and cancelling the confirmation
  returns to the list. Native content frames and the clipping radius stay aligned when the window
  resizes.

- Account-card layout in Chrome on macOS, in English and Simplified Chinese, light and dark: the account
  identity, tags and More do not overlap, including long names and emails. Hover and
  keyboard focus highlight switchable cards; identity activation opens the confirmation and Escape
  returns focus to it.

- Update sheets in Chrome on macOS, in both languages and appearances: long notes, an empty account
  list, determinate and indeterminate download progress, Escape/Later dismissal, and exactly one
  installation request after confirmation. Keyboard focus stays within each sheet, including its
  scrollable notes, then returns to the account list when dismissed.
- Official Tauri updater loopback tests: valid signatures, tampered payload rejection, signed-version
  mismatch rejection, current/older release filtering and HTTP failures. The macOS installer replaces
  an isolated synthetic bundle and leaves account data outside it intact; an invalid archive leaves
  that bundle intact.
- A locally built macOS archive signed with the production updater key: signature and signed version
  verified through the official updater, installed into a temporary bundle, then code signature and
  bundle version checked. No installed app was launched or replaced.
- End-to-end updating of a running installed OrbitSwitch and native Windows installation are **not
  verified**. The isolated macOS test does not verify restart, login-item survival or Gatekeeper.

## Known limitations

Accepted limitations are tracked as issues labeled
[`deferred`](https://github.com/shengyy/orbitswitch/issues?q=is%3Aissue+is%3Aopen+label%3Adeferred), each
saying why it waits and what would bring it back. They are not repeated here.
