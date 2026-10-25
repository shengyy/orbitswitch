// Interactive 3-tab product demo (fictional @example.com accounts only)
// and optional English / Simplified Chinese switch for the landing page.

const PLATFORMS = {
  gemini: {
    id: "gemini",
    tabLabel: "Gemini",
    clientName: "Antigravity",
    cliName: "agy",
    activeId: "ada",
    statusEn: "Antigravity is running",
    statusZh: "Antigravity 运行中",
    updatedEn: "Updated 1m ago",
    updatedZh: "1 分钟前更新",
    confirmBodyEn:
      "Antigravity will close and reopen signed in to this account. Running agent tasks will stop.",
    confirmBodyZh: "Antigravity 会先关闭，再以该账号重新打开。进行中的 Agent 任务会中断。",
    processesEn: "Closes: Antigravity · language server · helpers · agy CLI",
    processesZh: "将结束：Antigravity · 语言服务 · 辅助进程 · agy CLI",
    addingBodyEn:
      "Choose the Google account to add in your browser and allow access. OrbitSwitch picks it up automatically.",
    addingBodyZh: "在浏览器中选择要添加的 Google 账号并完成授权，OrbitSwitch 会自动接收。",
    accounts: [
      {
        id: "ada",
        name: "Ada Lovelace",
        email: "ada@example.com",
        plan: { kind: "pro", label: "PRO", title: "Google AI Pro" },
        groups: [
          {
            label: "Gemini",
            models: "Gemini Pro, Gemini Flash",
            fiveHour: { remaining: 0.62, resetEn: "3h 00m", resetZh: "3 小时" },
            weekly: { remaining: 0.88, resetEn: "4d 0h", resetZh: "4 天" },
          },
          {
            label: "Claude",
            models: "Claude Sonnet, Claude Opus, GPT-OSS",
            fiveHour: { remaining: 0.07, resetEn: "1h 12m", resetZh: "1 小时 12 分" },
            weekly: { remaining: 0.41, resetEn: "4d 0h", resetZh: "4 天" },
          },
        ],
      },
      {
        id: "grace",
        name: "Grace Hopper",
        email: "grace@example.com",
        plan: { kind: "pro", label: "PRO", title: "Google AI Pro" },
        groups: [
          {
            label: "Gemini",
            models: "Gemini Pro, Gemini Flash",
            fiveHour: { remaining: 1.0, resetEn: "5h 00m", resetZh: "5 小时" },
            weekly: { remaining: 0.97, resetEn: "5d 10h", resetZh: "5 天 10 小时" },
          },
          {
            label: "Claude",
            models: "Claude Sonnet, Claude Opus, GPT-OSS",
            fiveHour: { remaining: 0.94, resetEn: "4h 00m", resetZh: "4 小时" },
            weekly: { remaining: 0.83, resetEn: "5d 10h", resetZh: "5 天 10 小时" },
          },
        ],
      },
      {
        id: "alan",
        name: "Alan Turing",
        email: "alan@example.com",
        plan: { kind: "ultra", label: "ULTRA", title: "Google AI Ultra" },
        groups: [
          {
            label: "Gemini",
            models: "Gemini Pro, Gemini Flash",
            fiveHour: { remaining: 0.74, resetEn: "2h 00m", resetZh: "2 小时" },
            weekly: { remaining: 0.9, resetEn: "1d 16h", resetZh: "1 天 16 小时" },
          },
          {
            label: "Claude",
            models: "Claude Sonnet, Claude Opus, GPT-OSS",
            fiveHour: { remaining: 0.31, resetEn: "3h 20m", resetZh: "3 小时 20 分" },
            weekly: { remaining: 0.58, resetEn: "1d 16h", resetZh: "1 天 16 小时" },
          },
        ],
      },
    ],
  },
  chatgpt: {
    id: "chatgpt",
    tabLabel: "ChatGPT",
    clientName: "Codex",
    cliName: "codex",
    activeId: "ada",
    statusEn: "Codex is ready",
    statusZh: "Codex 已就绪",
    updatedEn: "Updated just now",
    updatedZh: "刚刚更新",
    confirmBodyEn:
      "Codex desktop and the codex CLI will switch to this account after verifying its authorization.",
    confirmBodyZh: "校验目标账号授权后，Codex 桌面端与 codex CLI 将切换至该账号。",
    processesEn: "Affects: Codex app · codex CLI credentials",
    processesZh: "将同步：Codex 桌面端 · codex CLI 登录凭据",
    addingBodyEn:
      "Sign in with your OpenAI account in the browser. Your active Codex session stays untouched until you switch.",
    addingBodyZh: "在浏览器中登录你的 OpenAI 账号。在你确认切换前，当前 Codex 会话不会改变。",
    accounts: [
      {
        id: "ada",
        name: "Ada Lovelace",
        email: "ada@example.com",
        plan: { kind: "pro", label: "PRO", title: "ChatGPT Pro" },
        groups: [
          {
            label: "Codex",
            models: "Shared Codex coding pool",
            fiveHour: null,
            weekly: { remaining: 0.64, resetEn: "3d 12h", resetZh: "3 天 12 小时" },
          },
        ],
      },
      {
        id: "grace",
        name: "Grace Hopper",
        email: "grace@example.com",
        plan: { kind: "pro", label: "PRO", title: "ChatGPT Pro" },
        groups: [
          {
            label: "Codex",
            models: "Shared Codex coding pool",
            fiveHour: null,
            weekly: { remaining: 0.91, resetEn: "6d 02h", resetZh: "6 天 2 小时" },
          },
        ],
      },
      {
        id: "alan",
        name: "Alan Turing",
        email: "alan@example.com",
        plan: { kind: "plus", label: "PLUS", title: "ChatGPT Plus" },
        groups: [
          {
            label: "Codex",
            models: "Shared Codex coding pool",
            fiveHour: { remaining: 0.52, resetEn: "2h 15m", resetZh: "2 小时 15 分" },
            weekly: { remaining: 0.44, resetEn: "2d 08h", resetZh: "2 天 8 小时" },
          },
        ],
      },
    ],
  },
  claude: {
    id: "claude",
    tabLabel: "Claude",
    clientName: "Claude Code",
    cliName: "claude",
    activeId: "ada",
    statusEn: "Claude Code is active",
    statusZh: "Claude Code 已连接",
    updatedEn: "Updated 2m ago",
    updatedZh: "2 分钟前更新",
    confirmBodyEn:
      "Claude Code and its local credentials will switch to this account after confirmation and verification.",
    confirmBodyZh: "确认并完成凭据校验后，Claude Code 本地登录状态将切换至该账号。",
    processesEn: "Affects: Claude Code · claude CLI & desktop sign-in",
    processesZh: "将同步：Claude Code · claude CLI 与桌面端登录凭据",
    addingBodyEn:
      "Complete Anthropic sign-in in your browser to add another personal Claude account to OrbitSwitch.",
    addingBodyZh: "在浏览器中完成 Anthropic 登录，即可将另一个本人 Claude 账号添加至 OrbitSwitch。",
    accounts: [
      {
        id: "ada",
        name: "Ada Lovelace",
        email: "ada@example.com",
        plan: { kind: "max", label: "MAX", title: "Claude Max" },
        groups: [
          {
            label: "Sonnet",
            models: "Claude Sonnet",
            fiveHour: { remaining: 0.28, resetEn: "54m", resetZh: "54 分钟" },
            weekly: { remaining: 0.72, resetEn: "2d 18h", resetZh: "2 天 18 小时" },
          },
          {
            label: "Opus",
            models: "Claude Opus",
            fiveHour: { remaining: 0.08, resetEn: "54m", resetZh: "54 分钟" },
            weekly: { remaining: 0.39, resetEn: "2d 18h", resetZh: "2 天 18 小时" },
          },
        ],
      },
      {
        id: "alan",
        name: "Alan Turing",
        email: "alan@example.com",
        plan: { kind: "max", label: "MAX", title: "Claude Max" },
        groups: [
          {
            label: "Sonnet",
            models: "Claude Sonnet",
            fiveHour: { remaining: 0.95, resetEn: "4h 30m", resetZh: "4 小时 30 分" },
            weekly: { remaining: 0.89, resetEn: "5d 04h", resetZh: "5 天 4 小时" },
          },
          {
            label: "Opus",
            models: "Claude Opus",
            fiveHour: { remaining: 0.86, resetEn: "4h 30m", resetZh: "4 小时 30 分" },
            weekly: { remaining: 0.81, resetEn: "5d 04h", resetZh: "5 天 4 小时" },
          },
        ],
      },
      {
        id: "grace",
        name: "Grace Hopper",
        email: "grace@example.com",
        plan: { kind: "pro", label: "PRO", title: "Claude Pro" },
        groups: [
          {
            label: "Sonnet",
            models: "Claude Sonnet",
            fiveHour: { remaining: 0.68, resetEn: "3h 10m", resetZh: "3 小时 10 分" },
            weekly: { remaining: 0.55, resetEn: "4d 06h", resetZh: "4 天 6 小时" },
          },
        ],
      },
    ],
  },
};

const I18N = {
  zh: {
    "nav.platforms": "三平台",
    "nav.workflow": "切换机制",
    "nav.boundaries": "安全边界",
    "nav.install": "安装",
    "nav.faq": "常见问题",
    "nav.lang": "EN",
    "nav.langLabel": "Switch website language to English",
    "hero.eyebrow": "三个平台，一个轻巧的切换器。",
    "hero.title": "切账号，看额度，不断流。",
    "hero.lede":
      "OrbitSwitch 常驻 macOS 菜单栏与 Windows 系统托盘，在 Gemini（Antigravity）、ChatGPT（Codex）、Claude（Claude Code）三个标签下管理你的多个本人账号：一眼看清订阅套餐、实际额度窗口及重置时间，确认后再安全切换。",
    "hero.pillar1.title": "Switch accounts",
    "hero.pillar1.desc": "添加本人账号，关联客户端，确认后切换。",
    "hero.pillar2.title": "See quota",
    "hero.pillar2.desc": "按账号实际窗口展示剩余比例、重置倒计时与推荐账号。",
    "hero.pillar3.title": "Stay in flow",
    "hero.pillar3.desc": "切换前先校验授权并点名受影响进程，写入失败自动回滚原凭据。",
    "hero.dlMac": "下载 macOS 版",
    "hero.dlWin": "下载 Windows 版",
    "hero.dlReleases": "查看所有版本",
    "hero.spec": "macOS 26+（Apple silicon 与 Intel）· Windows 10/11 x64 · MIT 开源免费",
    "demo.sceneList": "账号与额度",
    "demo.sceneConfirm": "切换确认层",
    "demo.sceneAdd": "浏览器添加账号",
    "demo.badge": "交互式产品演示 · 虚构 @example.com 账号与演示额度",
    "platforms.eyebrow": "三主标签 · 三个编程环境",
    "platforms.title": "同一面板，覆盖三大 AI 编程工具。",
    "platforms.lede":
      "每个主标签对应一套独立的本地编程客户端与 CLI。OrbitSwitch 只读取与展示各编程工具自身的额度窗口，不将网页聊天额度混为一谈。",
    "platforms.gemini.sub": "Google Antigravity 桌面端与 agy CLI",
    "platforms.gemini.status": "macOS 已真机验证 · Windows CI 构建",
    "platforms.gemini.p1":
      "自动识别当前已登录的 Antigravity 账号，支持 Google AI Pro / Ultra / Free 套餐标识。",
    "platforms.gemini.p2": "并排显示 Gemini 与 Claude/GPT 模型组的 5 小时及周度剩余比例与重置时间。",
    "platforms.gemini.p3":
      "确认后依次校验目标授权、结束 Antigravity 与 agy CLI、写入并校验新凭据，失败自动恢复原登录。",
    "platforms.chatgpt.sub": "OpenAI Codex 桌面端与 codex CLI",
    "platforms.chatgpt.status": "身份与额度读取已验证 · 切换 beta",
    "platforms.chatgpt.p1": "按本人账号管理 ChatGPT Plus / Pro 订阅下的 Codex 编程额度窗口。",
    "platforms.chatgpt.p2": "按实际返回展示额度与恢复时间；仅有周限额的账号不显示 5 小时窗口。",
    "platforms.chatgpt.p3": "经用户确认后同步切换本地 Codex 桌面端与 codex CLI 凭据。",
    "platforms.claude.sub": "Anthropic Claude Code 与 claude CLI",
    "platforms.claude.status": "桌面关联验证通过 · 切换 beta",
    "platforms.claude.p1": "管理多个本人 Claude Pro / Max 账号，清晰区分当前使用与余量最多账号。",
    "platforms.claude.p2": "展示 Sonnet 与 Opus 等模型窗口的 5 小时与周度剩余额度及重置时间。",
    "platforms.claude.p3": "确认后切换本地 Claude Code 与桌面端凭据，异常时保留原账号不丢失。",
    "platforms.quotaNote.title": "编程工具额度 ≠ 网页端对话额度",
    "platforms.quotaNote.body":
      "OrbitSwitch 展示的是 Antigravity、Codex、Claude Code 各自报告的编程速率窗口。Gemini 网页版或 ChatGPT 网页聊天的消费配额遵循各自独立规则，不等同于编程客户端额度。",
    "workflow.eyebrow": "克制而可靠的切换时序",
    "workflow.title": "每一次切换，都由你确认、可回滚。",
    "workflow.lede":
      "你可能有正在运行的 Agent 任务。OrbitSwitch 绝不在后台静默切号，也不会在未校验目标凭据前改动本地状态。",
    "workflow.s1.title": "一眼识别余量最多账号",
    "workflow.s1.body":
      "面板打开或后台每 5 分钟刷新额度。低于 35% 呈琥珀色提醒，低于 10% 呈红色告警，自动标记最值得切换的账号。",
    "workflow.s2.title": "面板内上滑确认并点名进程",
    "workflow.s2.body":
      "点击目标账号先弹出内嵌确认层（Enter 确认，Esc 取消），明确列出将关闭或更新的客户端、语言服务与 CLI。",
    "workflow.s3.title": "先校验授权，失败原样恢复",
    "workflow.s3.body":
      "改动任何本地凭据前先确认目标账号授权有效；若写入或校验新登录状态失败，自动恢复原账号凭据。",
    "workflow.s4.title": "桌面端与 CLI 保持同步",
    "workflow.s4.body": "切换完成后重新拉起或同步对应客户端，让你立即回到编码心流。",
    "boundaries.eyebrow": "凭据留在本机",
    "boundaries.title": "自己的账号，自己掌控。",
    "boundaries.lede": "一个轻巧的本地工具。使用官方登录，每次切换由你确认，剩余额度随时可见。",
    "boundaries.b1.title": "系统凭据库存储",
    "boundaries.b1.body":
      "保存的账号凭据和桌面登录会话保存在 macOS 钥匙串或 Windows 凭据管理器，令牌与 Cookie 不会进入界面。",
    "boundaries.b2.title": "无云端、无代理、无遥测",
    "boundaries.b2.body":
      "没有自建服务器，不转发任何模型请求，不植入统计或跟踪脚本，仅直接请求官方接口与 GitHub Releases。",
    "boundaries.b3.title": "不自动轮询切号、不共享账号",
    "boundaries.b3.body":
      "专为开发者管理本人名下的多个账号设计。不提供 API 中继，不在额度耗尽时后台自动切号绕过限制。",
    "boundaries.b4.title": "诚实呈现验证范围",
    "boundaries.b4.body": "身份与额度读取在真实安装上核对，完整账号切换的验证范围单独记录在项目状态文档。",
    "install.eyebrow": "快速上手",
    "install.title": "三步安装 OrbitSwitch",
    "install.s1.title": "获取安装包",
    "install.s1.body":
      "下载适用于 macOS 26+ 的通用 .dmg（Apple silicon 与 Intel）或 Windows 10/11 x64 的 -setup.exe 安装程序。请确保已安装对应的编程工具。",
    "install.s2.title": "首次打开（未签名构建说明）",
    "install.s2.body1":
      "当前构建尚未配置 Apple Developer ID 或 Windows 商业代码签名证书。在 macOS 上可右键应用选择“打开”，或运行：",
    "install.s2.body2": "在 Windows SmartScreen 提示中选择“更多信息 → 仍要运行”。",
    "install.s3.title": "从菜单栏或托盘启动",
    "install.s3.body":
      "从菜单栏或托盘打开图标，选择平台，添加或导入账号。Claude Desktop 需先与 Claude Code 登录同一账号和工作区，再导入关联一次。",
    "faq.eyebrow": "细节解答",
    "faq.title": "常见问题",
    "faq.q1": "可以管理哪些客户端？",
    "faq.a1":
      "Gemini 对应 Antigravity，ChatGPT 对应 Codex，Claude 对应 Claude Desktop 和 Claude Code。每个标签独立保存账号、当前登录和额度。",
    "faq.q2": "怎样添加账号？",
    "faq.a2":
      "「添加」打开官方浏览器登录，不改变当前 CLI 账号。要同时管理 Claude Desktop，请先与 Claude Code 登录同一账号和工作区，再导入关联。",
    "faq.q3": "Gemini 网页版或 ChatGPT 聊天额度与这里的额度是一回事吗？",
    "faq.a3":
      "不是。OrbitSwitch 读取的是各编程客户端（Antigravity、Codex、Claude Code）对应的速率限制窗口（5 小时与周度余量），不代表网页端日常对话配额。",
    "faq.q4": "为什么切换账号时需要确认，而不是无感热切换？",
    "faq.a4":
      "确认后会关闭受影响的桌面与 CLI 进程，写入并验证账号，再打开桌面应用。CLI 会话需手动重开；凭据写入或校验失败会恢复原登录。",
    "faq.q5": "目前各平台与客户端的验证进度如何？",
    "faq.a5":
      "Antigravity 切换已在 macOS 验证。Codex、Claude 的身份与额度读取以及 Claude Desktop 关联通过了只读验证；真实 Codex/Claude 切换及 Windows 客户端行为仍未验证，详见 docs/status.md。",
    "footer.source": "GitHub 源码",
    "footer.releases": "版本发布",
    "footer.security": "安全说明",
    "footer.readmeZh": "中文文档",
    "footer.fine":
      "OrbitSwitch 是独立的开源项目（MIT License），与 Google、OpenAI 或 Anthropic 无隶属或背书关系。Antigravity、Gemini、ChatGPT、Codex、Claude 及 Claude Code 均为各自所有者的商标。",
  },
};

const EN_DEFAULTS = {};

function avatarHue(id) {
  let sum = 7;
  for (const ch of id) {
    sum = (sum * 31 + ch.charCodeAt(0)) % 360;
  }
  return sum;
}

function severity(remaining) {
  if (remaining < 0.1) return "critical";
  if (remaining < 0.35) return "low";
  return "ok";
}

function bestAccountId(accounts, activeId) {
  const windows = (group) => [group.fiveHour, group.weekly].filter(Boolean);
  const shape = (acc) =>
    acc.groups
      .map((g) => `${g.label}:${g.fiveHour ? "5h" : ""}:${g.weekly ? "week" : ""}`)
      .sort()
      .join("|");
  const score = (acc) => {
    const available = (g) => Math.min(...windows(g).map((w) => w.remaining));
    const primary = acc.groups.find((g) => g.label !== "Gemini");
    const gemini = acc.groups.find((g) => g.label === "Gemini");
    return available(primary ?? gemini) + (gemini ? available(gemini) / 1000 : 0);
  };
  const active = accounts.find((a) => a.id === activeId);
  const activeScore = active ? score(active) : 0;
  let bestId = null;
  let bestScore = activeScore + 0.05;
  for (const acc of accounts) {
    if (acc.id === activeId || (active && shape(acc) !== shape(active))) continue;
    const s = score(acc);
    if (s > bestScore) {
      bestScore = s;
      bestId = acc.id;
    }
  }
  return bestId;
}

function lowRecoveryHint(account, lang) {
  const candidates = [];
  for (const group of account.groups) {
    if (group.fiveHour && group.fiveHour.remaining < 0.35) {
      candidates.push({
        label: `${group.label} ${lang === "zh" ? "5 小时" : "5h"}`,
        remaining: group.fiveHour.remaining,
        reset: lang === "zh" ? group.fiveHour.resetZh : group.fiveHour.resetEn,
      });
    }
    if (group.weekly && group.weekly.remaining < 0.35) {
      candidates.push({
        label: `${group.label} ${lang === "zh" ? "本周" : "Week"}`,
        remaining: group.weekly.remaining,
        reset: lang === "zh" ? group.weekly.resetZh : group.weekly.resetEn,
      });
    }
  }
  candidates.sort((a, b) => a.remaining - b.remaining);
  if (!candidates.length) return null;
  const first = candidates[0];
  return lang === "zh"
    ? `${first.label} · ${first.reset}后重置`
    : `${first.label} · resets in ${first.reset}`;
}

const demoState = {
  lang: "en",
  tab: "gemini",
  scene: "list", // "list" | "confirm" | "switching" | "adding" | "done"
  targetAccountId: "grace",
  switchStep: 0,
  stepTimer: null,
  refreshing: false,
};

function clearStepTimer() {
  if (demoState.stepTimer) {
    clearTimeout(demoState.stepTimer);
    demoState.stepTimer = null;
  }
}

function getPlatform() {
  return PLATFORMS[demoState.tab] || PLATFORMS.gemini;
}

function getTargetAccount(platform) {
  const nonActive = platform.accounts.filter((a) => a.id !== platform.activeId);
  const bestId = bestAccountId(platform.accounts, platform.activeId);
  return (
    nonActive.find((a) => a.id === demoState.targetAccountId) ||
    nonActive.find((a) => a.id === bestId) ||
    nonActive[0] ||
    platform.accounts[0]
  );
}

function renderDemo() {
  const lang = demoState.lang;
  const platform = getPlatform();
  const root = document.getElementById("demo-popover");
  if (!root) return;

  // Update tab buttons
  for (const btn of document.querySelectorAll("[data-demo-tab]")) {
    const isSelected = btn.dataset.demoTab === platform.id;
    btn.setAttribute("aria-selected", String(isSelected));
    btn.tabIndex = isSelected ? 0 : -1;
    btn.classList.toggle("is-active", isSelected);
  }

  // Update scene pills
  for (const pill of document.querySelectorAll("[data-demo-scene]")) {
    const activeScene =
      demoState.scene === "switching" || demoState.scene === "done" ? "confirm" : demoState.scene;
    const isCurrent = pill.dataset.demoScene === activeScene;
    pill.classList.toggle("is-active", isCurrent);
    pill.setAttribute("aria-pressed", String(isCurrent));
  }

  // Header status
  const statusEl = document.getElementById("demo-status-text");
  if (statusEl) {
    const statusText = lang === "zh" ? platform.statusZh : platform.statusEn;
    const updatedText = demoState.refreshing
      ? lang === "zh"
        ? "正在刷新…"
        : "Refreshing…"
      : lang === "zh"
        ? platform.updatedZh
        : platform.updatedEn;
    statusEl.textContent = `${statusText} · ${updatedText}`;
  }

  const refreshBtn = document.getElementById("demo-refresh-btn");
  if (refreshBtn) {
    refreshBtn.classList.toggle("is-spinning", demoState.refreshing);
    refreshBtn.setAttribute("aria-label", lang === "zh" ? "刷新额度" : "Refresh quotas");
  }

  // Footer labels
  const addBtnLabel = document.getElementById("demo-add-label");
  if (addBtnLabel) addBtnLabel.textContent = lang === "zh" ? "添加账号" : "Add account";
  const restartBtnLabel = document.getElementById("demo-restart-label");
  if (restartBtnLabel) restartBtnLabel.textContent = lang === "zh" ? "重启" : "Restart";
  const quitBtnLabel = document.getElementById("demo-quit-label");
  if (quitBtnLabel) quitBtnLabel.textContent = lang === "zh" ? "结束" : "Quit";

  // Sort accounts: active first
  const sorted = [...platform.accounts].sort(
    (a, b) => Number(b.id === platform.activeId) - Number(a.id === platform.activeId),
  );
  const bestId = bestAccountId(platform.accounts, platform.activeId);

  const listEl = document.getElementById("demo-account-list");
  if (listEl) {
    listEl.setAttribute("aria-labelledby", `tab-${platform.id}`);
    listEl.innerHTML = sorted
      .map((acc) => {
        const isActive = acc.id === platform.activeId;
        const isBest = !isActive && acc.id === bestId;
        const hue = avatarHue(acc.id);
        const initial = acc.name.charAt(0).toUpperCase();
        const hint = lowRecoveryHint(acc, lang);
        const currentLabel = lang === "zh" ? "当前" : "Current";
        const bestLabel = lang === "zh" ? "余量最多" : "Most left";
        const fiveLabel = lang === "zh" ? "5 小时" : "5h";
        const weekLabel = lang === "zh" ? "本周" : "Week";
        const actionHint = isActive
          ? acc.email
          : lang === "zh"
            ? `点击切换到 ${acc.email}`
            : `Click to switch to ${acc.email}`;

        const rowsHtml = acc.groups
          .map((g) => {
            const meters = [
              [fiveLabel, g.fiveHour],
              [weekLabel, g.weekly],
            ]
              .filter(([, window]) => window)
              .map(([label, window]) => {
                const pct = Math.round(window.remaining * 100);
                const reset = lang === "zh" ? `${window.resetZh}后重置` : `Resets in ${window.resetEn}`;
                return `<div class="demo-meter demo-meter-${severity(window.remaining)}" title="${reset}">
                  <span class="demo-meter-label">${label}</span>
                  <span class="demo-meter-track"><span class="demo-meter-fill" style="width: ${pct}%"></span></span>
                  <span class="demo-meter-value">${pct}%</span>
                </div>`;
              })
              .join("");
            return `
              <div class="demo-quota-row" title="${g.models}">
                <span class="demo-quota-group">${g.label}</span>
                ${meters}
              </div>
            `;
          })
          .join("");

        return `
          <article class="demo-card ${isActive ? "is-current" : "is-switchable"}" data-account-card="${acc.id}">
            <div class="demo-card-head">
              <button
                type="button"
                class="demo-card-identity"
                ${isActive ? "disabled" : ""}
                data-switch-account="${acc.id}"
                aria-label="${acc.name} · ${acc.plan.label} · ${actionHint}"
              >
                <span class="demo-avatar" style="--hue: ${hue}" aria-hidden="true">${initial}</span>
                <span class="demo-card-title">
                  <span class="demo-name">${acc.name}</span>
                  <span class="demo-plan demo-plan-${acc.plan.kind}" title="${acc.plan.title}">${acc.plan.label}</span>
                  ${isActive ? `<span class="demo-tag demo-tag-current">${currentLabel}</span>` : ""}
                  ${isBest ? `<span class="demo-tag demo-tag-best">${bestLabel}</span>` : ""}
                </span>
                <span class="demo-email">${acc.email}</span>
              </button>
              <span class="demo-card-action-hint" aria-hidden="true">
                ${isActive ? "" : lang === "zh" ? "切换 →" : "Switch →"}
              </span>
            </div>
            <div class="demo-card-quota">
              ${rowsHtml}
              ${hint ? `<div class="demo-card-hint">${hint}</div>` : ""}
            </div>
          </article>
        `;
      })
      .join("");
  }

  // Sheet overlay rendering
  const sheetLayer = document.getElementById("demo-sheet-layer");
  if (!sheetLayer) return;

  if (demoState.scene === "list") {
    sheetLayer.hidden = true;
    sheetLayer.innerHTML = "";
    return;
  }

  sheetLayer.hidden = false;
  const target = getTargetAccount(platform);

  if (demoState.scene === "confirm") {
    const title = lang === "zh" ? `切换到 ${target.email}？` : `Switch to ${target.email}?`;
    const body = lang === "zh" ? platform.confirmBodyZh : platform.confirmBodyEn;
    const note = lang === "zh" ? platform.processesZh : platform.processesEn;
    const cancelText = lang === "zh" ? "取消" : "Cancel";
    const confirmText = lang === "zh" ? "切换并重启" : "Switch & Restart";

    sheetLayer.innerHTML = `
      <div class="demo-sheet" role="dialog" aria-modal="true" aria-labelledby="demo-sheet-title">
        <h4 id="demo-sheet-title" class="demo-sheet-title">${title}</h4>
        <p class="demo-sheet-body">${body}</p>
        <p class="demo-sheet-note">${note}</p>
        <div class="demo-sheet-actions">
          <button type="button" class="demo-sheet-btn" data-sheet-action="cancel">${cancelText}</button>
          <button type="button" class="demo-sheet-btn demo-sheet-btn-primary" data-sheet-action="confirm">${confirmText}</button>
        </div>
      </div>
    `;
  } else if (demoState.scene === "switching" || demoState.scene === "done") {
    const steps =
      lang === "zh"
        ? [
            "校验目标账号授权",
            `关闭 / 同步 ${platform.clientName}`,
            "写入并校验登录凭据",
            `重新打开 ${platform.clientName}`,
          ]
        : [
            "Checking authorization",
            `Closing ${platform.clientName}`,
            "Signing in & verifying",
            `Opening ${platform.clientName}`,
          ];
    const doneTitle = lang === "zh" ? `已切换到 ${target.name}` : `Now using ${target.name}`;
    const progressTitle = lang === "zh" ? `正在切换到 ${target.email}` : `Switching to ${target.email}`;

    sheetLayer.innerHTML = `
      <div class="demo-sheet" role="status" aria-live="polite">
        <h4 class="demo-sheet-title">${demoState.scene === "done" ? doneTitle : progressTitle}</h4>
        <ul class="demo-step-list">
          ${steps
            .map((label, idx) => {
              const stateClass =
                demoState.scene === "done" || idx < demoState.switchStep
                  ? "is-done"
                  : idx === demoState.switchStep
                    ? "is-current"
                    : "is-pending";
              return `<li class="demo-step ${stateClass}"><span class="demo-step-indicator"></span><span>${label}</span></li>`;
            })
            .join("")}
        </ul>
      </div>
    `;
  } else if (demoState.scene === "adding") {
    const title = lang === "zh" ? "在浏览器中继续" : "Continue in your browser";
    const body = lang === "zh" ? platform.addingBodyZh : platform.addingBodyEn;
    const cancelText = lang === "zh" ? "取消" : "Cancel";
    sheetLayer.innerHTML = `
      <div class="demo-sheet" role="dialog" aria-modal="true" aria-labelledby="demo-sheet-title">
        <div class="demo-spinner" aria-hidden="true"></div>
        <h4 id="demo-sheet-title" class="demo-sheet-title">${title}</h4>
        <p class="demo-sheet-body">${body}</p>
        <div class="demo-sheet-actions">
          <button type="button" class="demo-sheet-btn" data-sheet-action="cancel">${cancelText}</button>
        </div>
      </div>
    `;
  }
}

function runDemoSwitchSequence() {
  clearStepTimer();
  const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const platform = getPlatform();
  const target = getTargetAccount(platform);

  if (reducedMotion) {
    platform.activeId = target.id;
    demoState.scene = "list";
    renderDemo();
    return;
  }

  demoState.scene = "switching";
  demoState.switchStep = 0;
  renderDemo();

  const advance = () => {
    if (demoState.switchStep < 3) {
      demoState.switchStep += 1;
      renderDemo();
      demoState.stepTimer = setTimeout(advance, 320);
    } else {
      platform.activeId = target.id;
      demoState.scene = "done";
      renderDemo();
      demoState.stepTimer = setTimeout(() => {
        demoState.scene = "list";
        renderDemo();
      }, 900);
    }
  };

  demoState.stepTimer = setTimeout(advance, 320);
}

function setLanguage(lang) {
  demoState.lang = lang;
  document.documentElement.lang = lang === "zh" ? "zh-CN" : "en";

  for (const el of document.querySelectorAll("[data-i18n]")) {
    const key = el.dataset.i18n;
    if (!(key in EN_DEFAULTS)) {
      EN_DEFAULTS[key] = el.textContent;
    }
    el.textContent = lang === "zh" ? (I18N.zh[key] ?? EN_DEFAULTS[key]) : EN_DEFAULTS[key];
  }

  const langBtn = document.getElementById("lang-toggle");
  if (langBtn) {
    langBtn.setAttribute(
      "aria-label",
      lang === "zh" ? "Switch language to English" : "切换网站语言为简体中文",
    );
  }

  if (typeof window.__refreshReleaseCopy === "function") {
    window.__refreshReleaseCopy();
  }

  renderDemo();
}

document.addEventListener("DOMContentLoaded", () => {
  // Record English defaults from static HTML
  for (const el of document.querySelectorAll("[data-i18n]")) {
    EN_DEFAULTS[el.dataset.i18n] = el.textContent;
  }

  // Language toggle
  const langBtn = document.getElementById("lang-toggle");
  langBtn?.addEventListener("click", () => {
    setLanguage(demoState.lang === "en" ? "zh" : "en");
  });

  // 3-Tab click & keyboard navigation
  const tabButtons = Array.from(document.querySelectorAll("[data-demo-tab]"));
  for (const btn of tabButtons) {
    btn.addEventListener("click", () => {
      clearStepTimer();
      demoState.tab = btn.dataset.demoTab;
      demoState.scene = "list";
      const platform = getPlatform();
      const best = bestAccountId(platform.accounts, platform.activeId);
      demoState.targetAccountId =
        best || platform.accounts.find((a) => a.id !== platform.activeId)?.id || "grace";
      renderDemo();
    });

    btn.addEventListener("keydown", (event) => {
      const idx = tabButtons.indexOf(btn);
      if (idx === -1) return;
      let nextIdx = idx;
      if (event.key === "ArrowRight") nextIdx = (idx + 1) % tabButtons.length;
      else if (event.key === "ArrowLeft") nextIdx = (idx - 1 + tabButtons.length) % tabButtons.length;
      else if (event.key === "Home") nextIdx = 0;
      else if (event.key === "End") nextIdx = tabButtons.length - 1;
      else return;

      event.preventDefault();
      const targetTab = tabButtons[nextIdx];
      targetTab.focus();
      targetTab.click();
    });
  }

  // Scene control pills
  for (const pill of document.querySelectorAll("[data-demo-scene]")) {
    pill.addEventListener("click", () => {
      clearStepTimer();
      const nextScene = pill.dataset.demoScene;
      if (nextScene === "confirm") {
        const platform = getPlatform();
        demoState.targetAccountId = getTargetAccount(platform).id;
      }
      demoState.scene = nextScene;
      renderDemo();
    });
  }

  // Refresh button inside demo popover
  const refreshBtn = document.getElementById("demo-refresh-btn");
  refreshBtn?.addEventListener("click", () => {
    if (demoState.refreshing) return;
    demoState.refreshing = true;
    renderDemo();
    setTimeout(() => {
      demoState.refreshing = false;
      const platform = getPlatform();
      platform.updatedEn = "Updated just now";
      platform.updatedZh = "刚刚更新";
      renderDemo();
    }, 420);
  });

  // Add account button inside demo popover
  const addBtn = document.getElementById("demo-add-btn");
  addBtn?.addEventListener("click", () => {
    clearStepTimer();
    demoState.scene = "adding";
    renderDemo();
  });

  // Delegate clicks inside the demo popover (account cards and confirmation sheet buttons)
  const popover = document.getElementById("demo-popover");
  popover?.addEventListener("click", (event) => {
    const target = event.target instanceof Element ? event.target : null;
    if (!target) return;

    const actionBtn = target.closest("[data-sheet-action]");
    if (actionBtn) {
      const action = actionBtn.getAttribute("data-sheet-action");
      if (action === "cancel") {
        clearStepTimer();
        demoState.scene = "list";
        renderDemo();
      } else if (action === "confirm") {
        runDemoSwitchSequence();
      }
      return;
    }

    const switchCard = target.closest("[data-switch-account], .demo-card.is-switchable");
    if (switchCard && demoState.scene === "list") {
      const accountId =
        switchCard.getAttribute("data-switch-account") || switchCard.getAttribute("data-account-card");
      const platform = getPlatform();
      if (accountId && accountId !== platform.activeId) {
        demoState.targetAccountId = accountId;
        demoState.scene = "confirm";
        renderDemo();
      }
    }
  });

  // Escape & Enter keys when confirmation sheet is open in demo
  document.addEventListener("keydown", (event) => {
    if (demoState.scene === "confirm" || demoState.scene === "adding") {
      if (event.key === "Escape") {
        clearStepTimer();
        demoState.scene = "list";
        renderDemo();
      }
    }
  });

  renderDemo();
});
