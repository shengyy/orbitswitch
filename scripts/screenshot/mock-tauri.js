// Stands in for Tauri's IPC so the real UI renders demo accounts in a browser.
// Only fictional names and example.com addresses: these screenshots are public.
// Query: ?scene=list|confirm|switching

const params = new URLSearchParams(location.search);
const scene = params.get("scene") ?? "list";
const provider = params.get("provider") ?? "antigravity";
localStorage.setItem("orbitswitch.provider", provider);
// Screenshots are English regardless of the machine's language.
Object.defineProperty(Navigator.prototype, "language", { get: () => params.get("lang") ?? "en-US" });

const MIN = 60_000;
const HOUR = 60 * MIN;
const at = (ms) => new Date(Date.now() + ms).toISOString();
const group = (key, label, models, [fiveHour, weekly, fiveReset, weekReset]) => ({
  key,
  label,
  models,
  fiveHour: { remaining: fiveHour, resetAt: at(fiveReset), label: "5h", durationMinutes: 300 },
  weekly: { remaining: weekly, resetAt: at(weekReset), label: "weekly", durationMinutes: 10080 },
  extra: [],
});
const quota = (gemini, claude) => ({
  status: "ready",
  fetchedAt: at(-MIN),
  stale: false,
  groups: [
    group("gemini", "Gemini", ["Gemini Flash", "Gemini Pro"], gemini),
    group("claude", "Claude", ["Claude Opus", "Claude Sonnet", "GPT-OSS"], claude),
  ],
});
const pro = { kind: "pro", name: "Google AI Pro" };

const snapshot = {
  platform: "macos",
  version: "0.1.0",
  providers: ["antigravity", "codex", "claude"].map((provider) => ({
    provider,
    activeId: provider === "antigravity" ? "ada" : `${provider}-ada`,
    installed: true,
    desktopInstalled: true,
    aligned: true,
    running: true,
  })),
  operation:
    scene === "switching"
      ? {
          kind: "switching",
          provider,
          targetId: provider === "antigravity" ? "grace" : `${provider}-grace`,
          step: "writing",
        }
      : { kind: "idle" },
  refreshing: false,
  refreshedAt: at(-MIN),
  update: { checking: false, available: null },
  accounts: [
    {
      provider: "antigravity",
      desktopReady: true,
      id: "ada",
      email: "ada@example.com",
      name: "Ada Lovelace",
      picture: null,
      plan: pro,
      quota: quota([0.62, 0.88, 3 * HOUR, 96 * HOUR], [0.07, 0.41, 72 * MIN, 96 * HOUR]),
    },
    {
      provider: "antigravity",
      desktopReady: true,
      id: "grace",
      email: "grace@example.com",
      name: "Grace Hopper",
      picture: null,
      plan: pro,
      quota: quota([1, 0.97, 5 * HOUR, 130 * HOUR], [0.94, 0.83, 4 * HOUR, 130 * HOUR]),
    },
    {
      provider: "antigravity",
      desktopReady: true,
      id: "alan",
      email: "alan@example.com",
      name: "Alan Turing",
      picture: null,
      plan: { kind: "ultra", name: "Google AI Ultra" },
      quota: quota([0.74, 0.9, 2 * HOUR, 40 * HOUR], [0.31, 0.58, 3 * HOUR + 20 * MIN, 40 * HOUR]),
    },
  ],
};

for (const p of ["codex", "claude"]) {
  const samples = snapshot.accounts.filter((account) => account.provider === "antigravity").slice(0, 2);
  snapshot.accounts.push(
    ...samples.map((account, index) => ({
      ...account,
      id: `${p}-${account.id}`,
      provider: p,
      plan: { kind: p === "codex" ? "plus" : "max", name: p === "codex" ? "ChatGPT Plus" : "Claude Max" },
      quota: {
        status: "ready",
        stale: false,
        fetchedAt: at(-MIN),
        groups: [
          group(
            p,
            p === "codex" ? "Codex" : "Claude",
            [],
            index ? [0.94, 0.83, HOUR, 96 * HOUR] : [0.08, 0.41, HOUR, 96 * HOUR],
          ),
        ],
      },
    })),
  );
}
if (scene === "stale") {
  const account = snapshot.accounts.find((a) => a.provider === provider);
  account.quota.stale = true;
  account.quota.groups[0].fiveHour.remaining = 0;
}
if (scene === "empty")
  snapshot.accounts = snapshot.accounts.filter((account) => account.provider !== provider);
if (scene === "binding")
  snapshot.accounts
    .filter((a) => a.provider === "claude")
    .forEach((a) => {
      a.desktopReady = false;
    });
window.__orbitDemo = { calls: [], snapshot };
const processes = [
  { pid: 1, kind: "app" },
  { pid: 2, kind: "languageServer" },
  { pid: 3, kind: "helper" },
];

let nextCallback = 1;
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { windowLabel: "main", label: "main" } },
  transformCallback: () => nextCallback++,
  invoke: async (command, args) => {
    window.__orbitDemo.calls.push({ command, args });
    if (command === "get_snapshot") return snapshot;
    if (command === "running_processes") return processes;
    return null;
  },
};

document.addEventListener("DOMContentLoaded", () => {
  if (document.querySelector(".clock")) document.querySelector(".clock").textContent = "Sun Oct 25  6:15 PM";
  if (scene !== "confirm") return;
  // Open the switch confirmation as soon as the recommended card renders.
  const observer = new MutationObserver(() => {
    const button = document.querySelector(".card:has(.tag-best) .card-identity");
    if (button) {
      observer.disconnect();
      button.click();
    }
  });
  observer.observe(document.body, { childList: true, subtree: true });
});
