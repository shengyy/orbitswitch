import type { Provider } from "./types";
export const PROVIDERS: Provider[] = ["antigravity", "codex", "claude"];
export const providerInfo = {
  antigravity: { label: "Gemini", client: "Antigravity", cli: "agy CLI" },
  codex: { label: "ChatGPT", client: "Codex", cli: "Codex CLI" },
  claude: { label: "Claude", client: "Claude", cli: "Claude Code" },
} satisfies Record<Provider, { label: string; client: string; cli: string }>;
