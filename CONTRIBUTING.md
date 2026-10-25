# Contributing

This page covers the human contributor workflow. Coding-agent rules live in [`AGENTS.md`](AGENTS.md),
and the module map in [`docs/architecture.md`](docs/architecture.md).

## Setup

- [Rust](https://rustup.rs) via rustup (pinned by [`rust-toolchain.toml`](rust-toolchain.toml) in the
  shared user-level toolchain).
- [Bun](https://bun.sh) for the frontend and Tauri CLI.
- macOS: Xcode Command Line Tools. Windows: MSVC build tools and WebView2. See Tauri's
  [prerequisites](https://tauri.app/start/prerequisites/).
- Installed official clients for real integration probes.

```bash
bun install
bun tauri dev
```

Keep `src-tauri/target/` between runs for incremental builds. To reset the build cache on demand:

```bash
cargo clean --manifest-path src-tauri/Cargo.toml
```

## Checks

CI runs these on macOS and Windows:

```bash
bun run check                       # Biome + TypeScript
bun run test                        # quota behavior
bun run build                       # frontend bundle
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

### Read-only checks against installed clients

```bash
cd src-tauri && cargo test system_tests -- --ignored --nocapture
```

Requires the target official clients to be signed in. Reads installed credentials, fetches quota and
validates Claude Desktop's binding without stopping processes or rewriting credentials. Update
[`docs/status.md`](docs/status.md) and the owning integration doc when verifying a new client version
or contract.

## Pull requests

`main` is protected; all changes land through pull requests:

1. Branch from `main` and open a **draft** PR (CI skips drafts).
2. Run the local checks above, then mark the PR **ready for review**.
3. Squash-merge once `gate` passes and the branch is up to date with `main`.

- One focused change per PR with a short imperative title.
- Record user-visible changes under `[Unreleased]` in `CHANGELOG.md`.
- Keep `README.md` and `README.zh-CN.md` in sync.
- Never include tokens, real account emails or personal paths.
- UI changes: run `bun scripts/screenshot/render.ts` and attach light/dark screenshots using fictional
  accounts.

## Issues

Issues are the single to-do list:

- `bug`, `enhancement`: checked against [PRODUCT.md](PRODUCT.md).
- `deferred`: accepted limitation with the reason it waits and what would reopen it.
- `needs-decision`: waiting on a product or design decision.
- `help wanted`, `good first issue`: open for contributors.

Release steps live in [`docs/release.md`](docs/release.md).
