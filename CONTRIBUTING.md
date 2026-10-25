# Contributing

Thanks for helping. This page is the human workflow; repository rules for coding agents live in
[`AGENTS.md`](AGENTS.md), and the code map in [`docs/architecture.md`](docs/architecture.md).

## Setup

- Install [Rust](https://rustup.rs) once per user through rustup. The root
  [`rust-toolchain.toml`](rust-toolchain.toml) registers this repository's version, components and
  installation profile with that shared installation. Rustup reuses the toolchain across repositories.
  Keep `CARGO_HOME` and `RUSTUP_HOME` at the workstation's user-level locations; do not create a separate
  Rust installation or download cache inside this repository.
- [Bun](https://bun.sh) for the frontend and the Tauri CLI.
- macOS: Xcode Command Line Tools. Windows: the MSVC build tools and WebView2 (preinstalled on Windows
  10/11). See Tauri's [prerequisites](https://tauri.app/start/prerequisites/).
- The relevant official client installed and signed in for real integration probes. Native account
  switching closes that client's sessions, so use disposable work and explicit confirmation.

```bash
bun install
bun tauri dev
```

The app appears in the menu bar (macOS) or the notification area (Windows). Running a second instance
just reveals the first, which is handy for opening the popover from a terminal.

### Rust caches

The workstation owns global Rust installation and cache settings. Use Cargo's built-in
[automatic global cache cleanup](https://doc.rust-lang.org/cargo/reference/config.html#global-caches)
with its defaults. It tracks downloaded dependencies; it does not clean this project's build output.

Keep `src-tauri/target/` between development and test runs so Cargo can reuse dependencies and
incremental compilation. When a deliberate build-cache reset is needed, stop active builds and use
the official [`cargo clean`](https://doc.rust-lang.org/cargo/commands/cargo-clean.html) command:

```bash
cargo clean --manifest-path src-tauri/Cargo.toml
```

This removes the build output and makes the next build cold. Run it on demand; build and deployment
commands must preserve the cache. Local installer and updater-package retention is covered in
[Local build](docs/release.md#local-build).

## Checks

The same commands run in CI on macOS and Windows:

```bash
bun run check                       # Biome + TypeScript
bun run test                        # quota behavior
bun run build                       # frontend bundle, needed before the Rust checks
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

### Read-only checks against installed clients

```bash
cd src-tauri && cargo test system_tests -- --ignored --nocapture
```

This requires all three official clients to be signed in. It reads installed credentials, fetches
quota and validates Claude Desktop's binding without launching or stopping clients or rewriting
their sign-ins. The Antigravity probe redeems its existing refresh token; native Codex/Claude probes
do not rotate tokens. Use a more specific test name to run an individual probe. If you verify a new
contract or client version, update [`docs/status.md`](docs/status.md) and the owning integration
document in the same PR. These probes do not establish end-to-end switching or enrollment.

## Pull requests

`main` is protected for everyone, maintainers included: no direct pushes, changes land through pull
requests only.

1. Branch from `main` and open a **draft** PR early. CI skips drafts.
2. When the checks above pass locally, mark the PR **ready for review**. CI then runs on macOS and
   Windows; the `gate` check must pass and the branch must contain the latest `main`.
3. Maintainers squash-merge.

- One focused change per PR, with a short imperative title.
- Add an entry under `[Unreleased]` in `CHANGELOG.md` for user-visible changes.
- Update both `README.md` and `README.zh-CN.md` when either changes.
- Never paste tokens, emails or log lines that contain them. Logs are designed not to include tokens;
  if you find one that does, that is a bug.
- UI changes: attach light and dark screenshots. `bun scripts/screenshot/render.ts` renders the real UI
  with fictional accounts; never post screenshots of real ones.

## Issues

Issues are the only to-do list; documents link to them instead of keeping their own.

- `bug`, `enhancement`: reports and requests. Check them against [PRODUCT.md](PRODUCT.md) first,
  especially its non-goals.
- `deferred`: known and accepted for now. The issue says why it waits and what would bring it back.
- `needs-decision`: waiting on a maintainer or user decision.
- `help wanted`, `good first issue`: good places to start.

Releases are cut by the maintainer (run end to end by their coding agent); see
[`docs/release.md`](docs/release.md).
