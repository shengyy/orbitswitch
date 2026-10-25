# Docs

The map of every document and directory in this repository, and the rules for writing them. Each
document below owns its subject; this page owns their routing. `docs/` holds only long-lived **current**
facts that every maintainer needs; history lives in Git, pull requests and [CHANGELOG.md](../CHANGELOG.md).

## Core documents

| Document | Only responsibility |
|---|---|
| [README.md](../README.md) / [README.zh-CN.md](../README.zh-CN.md) | Entry for users: what it is, install, use. The two are kept in sync |
| [PRODUCT.md](../PRODUCT.md) | What OrbitSwitch does, its rules and its non-goals |
| [CONTRIBUTING.md](../CONTRIBUTING.md) | Human workflow: setup, checks, issues, pull requests |
| [AGENTS.md](../AGENTS.md) | Shared boundaries and routing for coding agents, loaded every session |
| [CLAUDE.md](../CLAUDE.md) | Imports AGENTS.md and adds Claude-specific context |
| [SECURITY.md](../SECURITY.md) | Data inventory and storage locations, network sends, vulnerability reporting |
| [CHANGELOG.md](../CHANGELOG.md) | User-visible changes per version |
| [docs/README.md](README.md) | Document and directory responsibilities, routing and writing rules |
| [status.md](status.md) | Platform support and scope of feature verification |
| [architecture.md](architecture.md) | Code map, data flow, switch sequence, persistence mechanisms |
| [antigravity-integration.md](antigravity-integration.md) | Antigravity's external contracts and verified versions: credentials, OAuth, APIs, processes |
| [native-integrations.md](native-integrations.md) | Codex and Claude credentials, account/workspace binding, quota contracts and primary reference implementations |
| [design.md](design.md) | App and website visuals and interaction, icons and screenshots |
| [release.md](release.md) | Versioning, builds, release workflow, signing and website publishing |
| [pull_request_template.md](../.github/pull_request_template.md) | Pull request submission form; contributor workflow is in CONTRIBUTING.md |
| [LICENSE](../LICENSE) | MIT license terms |

## Directories

| Directory | Only holds |
|---|---|
| `src/` | React UI: snapshot-derived presentation, dialog state and requests through Tauri commands / APIs / plugins; no tokens |
| `src/components/` | UI components, shared icons and colocated component styles; sheets share `sheet.css` |
| `src/hooks/` | Snapshot subscription, content-driven sizing and relative-time updates |
| `src/styles/` | Shared design tokens and base UI styles |
| `src-tauri/` | The Rust app and its Tauri build / bundle configuration |
| `src-tauri/src/` | Backend modules; the module map is in [architecture.md](architecture.md) |
| `src-tauri/src/antigravity/` | Adapters for the local Antigravity installation, credentials, OAuth client and processes |
| `src-tauri/src/native/` | Codex and Claude subscription adapters, paired desktop sessions, process control and credential transactions |
| `src-tauri/src/google/` | Google OAuth, its loopback callback and Cloud Code API clients |
| `src-tauri/src/secret_store/` | Platform credential-store primitives |
| `src-tauri/capabilities/` | Tauri permissions for the app surface |
| `src-tauri/icons/` | Generated app and tray icons consumed by the Rust app and Tauri bundler |
| `site/` | Website HTML, styles, interactive fictional demo and release download resolver; publishing is in [release.md](release.md#website) |
| `assets/` | Public visual assets shared by the README and website |
| `assets/brand/` | Icon sources (SVG) |
| `assets/screenshots/` | Rendered screenshots for the README and website; regenerate, never hand-edit |
| `scripts/` | Development tools for generating visual assets and quota behavior tests |
| `scripts/screenshot/` | The screenshot renderer and its fictional demo data |
| `docs/` | Current maintainer reference documents and this responsibility map |
| `.github/` | GitHub automation and issue / pull request submission forms |
| `.github/workflows/` | CI, release and Pages workflows |
| `.github/ISSUE_TEMPLATE/` | Bug and feature forms, and issue-report routing |

Root files are the conventional public documents, license, app HTML entry point and toolchain / build
declarations. Code-module responsibilities are detailed in [architecture.md](architecture.md).

A new directory needs one responsibility and a row here in the same change.

## Writing rules

- **One owner per fact.** Other documents link to it instead of repeating it.
- **Current state only.** Documents describe how things are now. No plans, dated logs or "previously"
  notes; history is in Git, pull requests and the changelog.
- **Work items live in issues, not documents.** To-dos are open issues; accepted limitations carry the
  `deferred` label and say why they wait and what would bring them back; open questions carry
  `needs-decision`. Documents link to an issue or label instead of listing them.
- **Verified, then written.** Facts about Antigravity or a platform are recorded only after checking them
  on a real installation, with the version; anything unchecked is marked *not verified*.
- **Language.** User and contributor documents are in English; `AGENTS.md` and `CLAUDE.md` are in Chinese,
  the maintainers' working language. The two READMEs are kept in sync.
- **Names.** Files and directories use lowercase kebab-case, except the conventional root files
  (`README.md`, `AGENTS.md`, …) and names a language requires (Rust modules use snake_case).
- **Nothing private.** No tokens, real accounts, emails or absolute paths from someone's machine; use
  repository-relative paths and `<placeholder>`s.

## Context layers

| Layer | Loaded | Holds |
|---|---|---|
| `AGENTS.md` / `CLAUDE.md` | Every agent session | Boundaries that hold for every task, and routing |
| `PRODUCT.md`, `docs/**` | When a task needs them | The full rules and facts |

A rule lives in exactly one layer: `AGENTS.md` states the boundary and links to the owner of the detail.
