# Release

Versioning, builds, release workflow, signing and website publishing. Contributor checks live in
[CONTRIBUTING.md](../CONTRIBUTING.md).

## Version

`src-tauri/Cargo.toml` is the single version source; bundles, the app menu and the HTTP `User-Agent`
read it. Releases follow [SemVer](https://semver.org).

## Steps

Releases are run end to end with `gh`:

1. **Prepare in a PR.** Update [status.md](status.md), move `CHANGELOG.md` `[Unreleased]` entries under
   `## [X.Y.Z] - YYYY-MM-DD`, set `version` in `src-tauri/Cargo.toml`, run `cargo check` in
   `src-tauri` to update `Cargo.lock`, and merge through the draft → ready → `gate` flow.
2. **Build the draft.**

   ```bash
   gh workflow run release.yml --ref main -f version=X.Y.Z
   gh run watch "$(gh run list --workflow release.yml --limit 1 --json databaseId --jq '.[0].databaseId')" --exit-status
   ```

   `release.yml` verifies that the run is on `main`, matches `Cargo.toml` and is not yet released,
   then builds the universal macOS `.dmg`, Windows NSIS `-setup.exe`, updater archives, `.sig` files
   and `latest.json` on a draft release `vX.Y.Z`.
3. **Verify the draft.**

   ```bash
   gh release download vX.Y.Z --pattern '*.dmg' --dir <tmp>
   gh release view vX.Y.Z --json assets --jq '.assets[] | "\(.name) \(.digest)"'
   ```

   Mount the `.dmg` and confirm `codesign -dv` reports `io.github.shengyy.orbitswitch`, `lipo -archs`
   lists `x86_64 arm64`, and `CFBundleShortVersionString` is `X.Y.Z`. Download `latest.json`, the
   `.app.tar.gz`, `-setup.exe` and `.sig` files into `<tmp>`, save the release metadata as
   `release.json`, and run the artifact verifier:

   ```bash
   gh api 'repos/shengyy/orbitswitch/releases?per_page=100' --jq '.[] | select(.tag_name == "vX.Y.Z")' > <tmp>/release.json
   ORBITSWITCH_RELEASE_DIR=<tmp> cargo test --manifest-path src-tauri/Cargo.toml release_artifacts_verify -- --ignored
   ```
4. **Publish.**

   ```bash
   gh release edit vX.Y.Z --draft=false --latest --title "OrbitSwitch X.Y.Z" --notes-file <notes.md>
   gh api repos/shengyy/orbitswitch/releases/latest --jq .tag_name
   ```

## Signing

- **Platform signing**: macOS bundles are ad-hoc signed with `bundle.macOS.signingIdentity: "-"` so
  every build keeps the code identifier `io.github.shengyy.orbitswitch` required for `SMAppService`
  login items across updates. Windows builds are unsigned.
- **Updater signing**: `bundle.createUpdaterArtifacts` and `plugins.updater` bind updates to the
  public key in `src-tauri/tauri.conf.json` and require signed versions. The private key is stored in
  the `TAURI_SIGNING_PRIVATE_KEY` GitHub Actions secret.

## Local build

```bash
bun install
bun tauri build
bun tauri build --target universal-apple-darwin
```

Verify a locally signed macOS updater archive in an isolated temporary bundle:

```bash
ORBITSWITCH_UPDATE_PACKAGE=<bundle>/macos/OrbitSwitch.app.tar.gz cargo test --manifest-path src-tauri/Cargo.toml local_signed_bundle_installs -- --ignored
```

Remove local `bundle/` outputs and temporary release downloads once verification completes.

## Website

`site/` is deployed to <https://shengyy.github.io/orbitswitch/> by `.github/workflows/pages.yml` when
`site/` or `assets/` changes on `main`. Preview locally:

```bash
mkdir -p /tmp/orbitswitch-site/assets && cp -R site/. /tmp/orbitswitch-site/ \
  && cp -R assets/brand assets/screenshots /tmp/orbitswitch-site/assets/ \
  && python3 -m http.server 8000 --directory /tmp/orbitswitch-site
```
