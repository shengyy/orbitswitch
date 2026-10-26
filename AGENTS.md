# OrbitSwitch · Agent 指引

公开仓库（MIT）：在菜单栏 / 系统托盘切换 Antigravity、Codex、Claude 订阅账号并查看各账号额度。Tauri 2：Rust 后端执行全部副作用，React 前端从后端快照派生展示数据并请求操作。本文件每次会话都会加载，只放读代码推断不出、且每次任务都成立的边界；细则在下表的文档里，用到时再读。`CLAUDE.md` 导入本文件，人类流程见 [`CONTRIBUTING.md`](CONTRIBUTING.md)。

## 按需加载

| 范围 | 先读 |
|---|---|
| 产品规则与非目标 | [`PRODUCT.md`](PRODUCT.md) |
| 平台支持与功能验证范围 | [`docs/status.md`](docs/status.md) |
| 代码结构、数据流、切换时序、持久化机制 | [`docs/architecture.md`](docs/architecture.md) |
| Antigravity 的外部契约与验证版本：凭据、OAuth、接口、进程 | [`docs/antigravity-integration.md`](docs/antigravity-integration.md) |
| Codex / Claude 凭据、桌面绑定与额度契约 | [`docs/native-integrations.md`](docs/native-integrations.md) |
| 应用与官网的视觉、交互、图标、截图 | [`docs/design.md`](docs/design.md) |
| 版本、构建、发版、签名、官网发布 | [`docs/release.md`](docs/release.md) |
| 数据存储位置、网络发送与漏洞报告 | [`SECURITY.md`](SECURITY.md) |
| 文档与目录的职责、写入规则 | [`docs/README.md`](docs/README.md) |
| 待办、挂账、待确认 | GitHub Issues：挂账标 `deferred`，待确认标 `needs-decision` |

检查（CI 在 macOS 与 Windows 上跑全部）：文档 → `git diff --check`；`src/`、`site/`、`scripts/` → `bun run check`、`bun run test`、`bun run build`；`src-tauri/` → `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`；依赖客户端实机行为 → `cargo test system_tests -- --ignored --nocapture`；界面外观 → `bun scripts/screenshot/render.ts`。

## 约定

- 每个事实只有一个 owner，其它地方链接它：版本号在 `src-tauri/Cargo.toml`，命令名与事件名在 `src/ipc.ts`，前后端数据结构以 `src-tauri/src/model.rs` 为准、`src/types.ts` 跟随，两份 README 同步。
- 替代方案落地时，同一变更里删掉旧的日常读写路径及其测试和文档，仓库里只留一套做法。持久化标识或结构变更必须安全迁移旧数据：保留目标已有数据，写入并读回验证后再清理来源，失败可重试；在 `CHANGELOG.md` 交代升级行为。
- 只留最新事实：过期内容、旧口径、过程记录和已经不再是事实的旧文档、旧记录直接删除，不在仓库里留噪音，历史由 Git 追溯；文档与代码冲突时以代码为准修正文档。
- 单文件 ≤800 行正常，800–1200 行按职责考虑拆分，超过 1200 行在触碰时拆分或在文件头写明原因；一个目录一个职责，新目录登记到 `docs/README.md`。

## 本仓不变量

- 令牌只在 Rust 后端流转；前端从快照派生展示数据，通过命令与 Tauri 插件请求系统操作。
- 仓库公开：令牌、client secret、真实邮箱和个人路径只留在用户机器上，截图只用 `scripts/screenshot/` 的虚构账号。client secret 运行时从本机 Antigravity 读取；仓库里也避免 `GOCSPX-` 形状的字面量，它会触发 GitHub 密钥扫描。
- 客户端与平台行为先在真实安装上验证，再写进 `docs/antigravity-integration.md`、`docs/native-integrations.md` 与 `docs/status.md`；没验证的标 not verified。
- 结束任何受管客户端或改写其登录凭据前要由用户在界面上确认，写入失败要恢复原凭据，因为用户可能有正在运行的 Agent 任务。
- 支持面是 macOS 26+ 与 Windows 10/11 x64。macOS 包以 Bundle ID 为代码标识 ad-hoc 签名，登录项靠它在更新后继续生效。

## 协作与发版

- 中文回复，标识符、命令、路径保持英文。
- 在用户机器上真实切换账号、结束进程或改写凭据，以及改动仓库保护规则，先征得用户同意。
- `main` 受 "Protect main" ruleset 保护：主题分支先开 draft PR（CI 跳过），本机检查通过后转 ready，`gate` 通过且分支含最新 `main` 后 squash 合入。
- 用户可见的变化记入 `CHANGELOG.md` 的 `[Unreleased]`；已验证事实变化时同步 `docs/status.md`。
- 发版由你端到端执行：用户说发版后，按 [`docs/release.md`](docs/release.md) 用 `gh` 触发 Release、核对 draft、发布，用户不需要打开浏览器。
