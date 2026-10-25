<p align="center"><img src="assets/brand/app-icon.svg" width="104" alt="OrbitSwitch icon"></p>
<h1 align="center">OrbitSwitch</h1>
<p align="center">Switch accounts. See quota. Stay in flow.</p>
<p align="center"><a href="README.zh-CN.md">简体中文</a> · <a href="https://shengyy.github.io/orbitswitch/">Website</a> · <a href="https://github.com/shengyy/orbitswitch/releases">Releases</a></p>

A compact menu bar and system tray utility for **your own** Antigravity, Codex and Claude accounts.
Three tabs, one consistent view of subscriptions, remaining quota and reset times.

<picture><source media="(prefers-color-scheme: dark)" srcset="assets/screenshots/hero-dark.png"><img src="assets/screenshots/hero-light.png" width="640" alt="OrbitSwitch's three platform tabs and fictional subscription accounts"></picture>

| Tab | Account switching | Subscription quota |
|---|---|---|
| Gemini | Antigravity desktop + `agy` CLI | Google model groups |
| ChatGPT | Codex desktop + CLI | Codex windows and additional pools |
| Claude | Claude Desktop + Claude Code, after linking their sign-ins | Claude shared and model-specific windows |

Changing tabs only changes the view. A switch asks before closing clients, verifies the new sign-in,
restores credentials on failure, and reopens the installed desktop app. Reopen CLI sessions yourself.
A failed refresh keeps the previous numbers with a stale label. Missing windows are never guessed.
These are the coding clients' quotas, not Gemini or ChatGPT web-chat allowances.

The multi-platform implementation is **unreleased**. Published 0.1.x packages are the original
Antigravity-only AgyOrbit. Codex and Claude identity/quota reads and Claude Desktop binding have been
verified on macOS; real account switching and browser enrollment for these new adapters remain
**not verified**. See the exact [verification scope](docs/status.md).

## Install and use

macOS 26+ or Windows 10/11 x64. Install the official clients you want to manage; adding a Codex or
Claude account requires its official CLI. Download a compatible build from
[Releases](https://github.com/shengyy/orbitswitch/releases).

macOS builds use ad-hoc signing, without Apple notarization. If macOS blocks a trusted downloaded
build, use **Open** from the context menu or `xattr -dr com.apple.quarantine /Applications/OrbitSwitch.app`.
Windows builds are unsigned; SmartScreen may require **More info → Run anyway**.

1. Open OrbitSwitch from the menu bar or tray and choose a platform tab.
2. **Add account** opens the official sign-in without changing your current client account.
   **Import** saves the account already signed in on this computer.
3. For Claude Desktop, first sign into the same account and workspace in both Desktop and Claude Code,
   then import. The card's **Link desktop** action explains this step.
4. Pick another account, review the affected processes, and confirm the switch.

The app menu offers Open at Login, logs, update checks and Quit. Updates restart OrbitSwitch only.
The rename retains the existing application identifier and saved account storage.

## Local by design

Credentials stay in the Rust backend and the OS credential store. Claude's paired desktop session is
stored there too. The frontend receives account metadata and quota snapshots, never tokens or cookies.
OrbitSwitch contacts official authentication/usage services and GitHub for updates. It has no account
server, model proxy, automatic account rotation or telemetry. Read the [security inventory](SECURITY.md).

Development: [CONTRIBUTING.md](CONTRIBUTING.md). Product rules: [PRODUCT.md](PRODUCT.md).
Implementation and reference projects: [native integrations](docs/native-integrations.md).

MIT. Independent of Google, OpenAI and Anthropic; product names belong to their respective owners.
