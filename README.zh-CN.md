<p align="center"><img src="assets/brand/app-icon.svg" width="104" alt="OrbitSwitch 图标"></p>
<h1 align="center">OrbitSwitch</h1>
<p align="center">切账号，看额度，继续工作。</p>
<p align="center"><a href="README.md">English</a> · <a href="https://shengyy.github.io/orbitswitch/">官网</a> · <a href="https://github.com/shengyy/orbitswitch/releases">版本下载</a></p>

一个精致小巧的菜单栏 / 系统托盘工具，管理**你自己的** Antigravity、Codex 和 Claude 账号。
三个 Tab，用同一套方式查看订阅套餐、剩余额度与重置时间。

<picture><source media="(prefers-color-scheme: dark)" srcset="assets/screenshots/hero-dark.png"><img src="assets/screenshots/hero-light.png" width="640" alt="OrbitSwitch 三平台标签及虚构的订阅账号"></picture>

| Tab | 账号切换 | 订阅额度 |
|---|---|---|
| Gemini | Antigravity 桌面端 + `agy` CLI | Google 模型分组 |
| ChatGPT | Codex 桌面端 + CLI | Codex 限额窗口及额外额度池 |
| Claude | 关联登录后的 Claude Desktop + Claude Code | Claude 共享窗口及模型专属窗口 |

切换 Tab 只改变视图。切换账号前会确认要结束的客户端，写入后校验登录，失败时恢复原凭据，
然后打开已安装的桌面应用；CLI 会话需要手动重开。额度刷新失败会保留旧数据并标明过期，
未返回的额度窗口不会猜测。这些是编程客户端额度，不代表 Gemini 或 ChatGPT 网页聊天额度。

OrbitSwitch 带来三平台体验；旧的 0.1.x 安装包仍是仅支持 Antigravity 的 AgyOrbit。
Codex、Claude 的身份与额度读取，以及 Claude Desktop 关联身份验证已在 macOS 实机通过；
新增适配器的真实账号切换、浏览器添加登录仍**未验证**。详见[验证范围](docs/status.md)。

## 安装与使用

支持 macOS 26+、Windows 10/11 x64。请安装需要管理的官方客户端；添加 Codex 或 Claude 账号
还需要对应的官方 CLI。从 [Releases](https://github.com/shengyy/orbitswitch/releases) 获取适用构建。

macOS 使用 ad-hoc 签名，尚未经过 Apple 公证。如果系统拦截可信来源的安装包，可右键应用选择
**打开**，或运行 `xattr -dr com.apple.quarantine /Applications/OrbitSwitch.app`。
Windows 安装包未签名，SmartScreen 可能需要选择 **更多信息 → 仍要运行**。

1. 从菜单栏或系统托盘打开 OrbitSwitch，选择平台 Tab。
2. **添加账号**打开官方登录，不改变当前客户端账号；**导入当前**保存这台电脑上已登录的账号。
3. Claude Desktop 需要先与 Claude Code 登录同一账号和工作区，再导入。卡片上的
   **关联桌面**会说明这一步骤。
4. 选择另一个账号，核对受影响进程，确认后切换。

应用菜单提供开机启动、查看日志、检查更新和退出。应用更新只重启 OrbitSwitch。
改名保留原应用标识及账号存储位置。

## 凭据留在本机

凭据只在 Rust 后端与系统凭据库中流转，关联的 Claude 桌面会话也保存在系统凭据库。
前端只接收账号信息和额度快照，不接触令牌或 Cookie。OrbitSwitch 仅访问官方认证 / 额度服务
及 GitHub 更新服务；没有账号服务器、模型代理、自动切号或遥测。详见[安全说明](SECURITY.md)。

开发说明：[CONTRIBUTING.md](CONTRIBUTING.md)。产品规则：[PRODUCT.md](PRODUCT.md)。
实现与参考项目：[原生客户端适配](docs/native-integrations.md)。

MIT 开源。独立于 Google、OpenAI 和 Anthropic，相关产品名归各自所有者所有。
