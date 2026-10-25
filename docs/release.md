# Release

This page owns versioning, builds, releases, signing and website publishing. Contributor checks and PR
workflow live in [CONTRIBUTING.md](../CONTRIBUTING.md); app and website visuals live in
[design.md](design.md).

## Version

`src-tauri/Cargo.toml` is the only version source; the bundles, the app menu and the User-Agent read it.
Releases follow [SemVer](https://semver.org).

## Steps

Releases are run end to end by the maintainer's coding agent with `gh`; nobody needs to open the
browser. The maintainer decides *when*; the agent does the rest.

1. **Prepare in a normal PR.** Make sure [status.md](status.md) matches what this version was verified
   on, move `CHANGELOG.md`'s `[Unreleased]` entries under `## [X.Y.Z] - YYYY-MM-DD`, set `version` in
   `src-tauri/Cargo.toml` and run `cargo check` in `src-tauri` to update `Cargo.lock`. Merge it through
   the usual draft → ready → `gate` flow.
2. **Build the draft.**

   ```bash
   gh workflow run release.yml --ref main -f version=X.Y.Z
   gh run watch "$(gh run list --workflow release.yml --limit 1 --json databaseId --jq '.[0].databaseId')" --exit-status
   ```

   The `authorize` job refuses another branch, a version that differs from `Cargo.toml`, or one already
   released. The build makes a universal macOS `.dmg` and a Windows NSIS installer from `main`'s current
   commit and attaches them to a draft release `vX.Y.Z`, together with updater packages, `.sig` files
   and `latest.json`. Release notes come from that version's changelog section and are included in the
   update manifest. Nothing is pushed from a local machine.
3. **Check the draft.**

   ```bash
   gh release download vX.Y.Z --pattern '*.dmg' --dir <tmp>
   gh release view vX.Y.Z --json assets --jq '.assets[] | "\(.name) \(.digest)"'   # compare with shasum -a 256
   ```

   Mount the `.dmg` and confirm `codesign -dv` reports the identifier `io.github.shengyy.agyorbit`,
   `lipo -archs` lists `x86_64 arm64`, and `CFBundleShortVersionString` is `X.Y.Z`; the Windows
   `-setup.exe` must be attached too. Download `latest.json`, the `.app.tar.gz`, `-setup.exe` and their
   `.sig` files into the same temporary directory. Save the draft's GitHub release metadata alongside
   them as `release.json` (the updater action uses its API asset URLs):

   ```bash
   gh api 'repos/shengyy/orbitswitch/releases?per_page=100' --jq '.[] | select(.tag_name == "vX.Y.Z")' > <tmp>/release.json
   ```

   Verify every manifest asset belongs to this tag, both macOS
   architectures use the universal package, Windows uses NSIS, and signatures verify with the shipped
   public key and announced version:

   ```bash
   AGYORBIT_RELEASE_DIR=<tmp> cargo test --manifest-path src-tauri/Cargo.toml release_artifacts_verify -- --ignored
   ```
4. **Publish.** Write the release notes from the version's `CHANGELOG.md` section, then:

   ```bash
   gh release edit vX.Y.Z --draft=false --latest --title "OrbitSwitch X.Y.Z" --notes-file <notes.md>
   gh api repos/shengyy/orbitswitch/releases/latest --jq .tag_name   # must print vX.Y.Z
   ```

   Publishing creates the tag; the website's download buttons follow the latest release automatically.

## Signing

Builds are not signed with an Apple Developer ID or a Windows code-signing certificate yet.

macOS bundles are ad-hoc signed during bundling (`bundle.macOS.signingIdentity: "-"`), which gives every
build the code identifier `io.github.shengyy.agyorbit`. Keep it: macOS keys the SMAppService login item
to the code identity, and the linker's default ad-hoc signature changes it on every build.

First-launch instructions for Gatekeeper and SmartScreen live in the [README](../README.md#install).

## Updater signing

Tauri's updater signature is separate from platform code signing. `bundle.createUpdaterArtifacts`
enables it; `plugins.updater` owns the public key, version binding and HTTPS manifest endpoint.
The official release action combines both matrix builds into `latest.json`, including `darwin-aarch64`
and `darwin-x86_64` entries for the universal archive and `windows-x86_64` for NSIS.

The stable private key is held outside the repository and in the GitHub Actions secret
`TAURI_SIGNING_PRIVATE_KEY`. Keep a secure copy: losing or replacing this key prevents existing
installations from verifying future updates. Never commit it or include it in logs. For a local
signed build, set `TAURI_SIGNING_PRIVATE_KEY` to the private key file's path. Release builds sign the
app version automatically; hand-signing must use `--app-version X.Y.Z`.

## Local build

```bash
bun install
bun tauri build                 # host platform
bun tauri build --target universal-apple-darwin   # needs both Apple targets: rustup target add x86_64-apple-darwin
```

Verify a locally signed macOS package without launching it or touching your installed app:

```bash
ORBITSWITCH_UPDATE_PACKAGE=<bundle>/macos/OrbitSwitch.app.tar.gz cargo test --manifest-path src-tauri/Cargo.toml local_signed_bundle_installs -- --ignored
```

The test downloads through loopback, verifies the real signature and version, replaces an isolated
synthetic bundle, then checks its code signature and bundle version.

Host-platform artifacts land in `src-tauri/target/release/bundle/`; builds with `--target` use
`src-tauri/target/<target>/release/bundle/`.

Published installers and updater packages are retained on GitHub Releases. Local bundle outputs and
temporary release downloads are disposable verification artifacts: keep them through smoke, signature
and installation checks, and through upload and verification of any requested delivery. Once those
checks pass, remove the corresponding bundle directory and temporary downloads, including downloaded
manifests, release metadata and signatures. Developer machines do not retain historical delivery
packages. Keep the installed application, signing keys and Cargo build cache outside those output
directories intact; [Rust cache maintenance](../CONTRIBUTING.md#rust-caches) has its own lifecycle.

## Website

`site/` is the landing page at <https://shengyy.github.io/orbitswitch/>. The `pages` workflow deploys it on
every push to `main` that touches `site/` or `assets/`, copying `assets/brand` and `assets/screenshots`
next to it. The download buttons ask GitHub's API for the latest release and pick its `.dmg` and
`-setup.exe`, so a new release needs no site change. The site's visual rules are in
[design.md](design.md#website). Preview locally:

```bash
mkdir -p /tmp/agyorbit-site/assets && cp -R site/. /tmp/agyorbit-site/ \
  && cp -R assets/brand assets/screenshots /tmp/agyorbit-site/assets/ \
  && python3 -m http.server 8000 --directory /tmp/agyorbit-site
```
