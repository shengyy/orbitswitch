// Derived quota facts used by several components.

import type { Account, QuotaGroup, QuotaWindow } from "./types";

export type Severity = "ok" | "low" | "critical";

export function severity(remaining: number): Severity {
  if (remaining < 0.1) return "critical";
  if (remaining < 0.35) return "low";
  return "ok";
}

/** The tightest reported window; absent windows do not imply zero or unlimited quota. */
export function available(group: QuotaGroup): number | null {
  const windows = [group.fiveHour, group.weekly, ...group.extra].filter((w): w is QuotaWindow => w !== null);
  return windows.length ? Math.min(...windows.map((w) => w.remaining)) : null;
}

/** Primary pool (`codex` or `claude`) decides; `gemini` breaks ties. */
function score(account: Account): number | null {
  if (account.quota.status !== "ready" || account.quota.stale) return null;
  const groups = account.quota.groups;
  const primaryGroup = groups.find((g) => g.key === (account.provider === "codex" ? "codex" : "claude"));
  const gemini = groups.find((g) => g.key === "gemini");
  const primary = (primaryGroup && available(primaryGroup)) ?? (gemini && available(gemini));
  if (primary === null || primary === undefined) return null;
  return primary + ((gemini && available(gemini)) ?? 0) / 1000;
}

/** The other account worth switching to, if it clearly beats the current one. */
export function bestAccountId(accounts: Account[], activeId: string | null): string | null {
  const active = accounts.find((a) => a.id === activeId);
  const activeScore = active ? score(active) : 0;
  if (activeScore === null) return null;
  let best: { id: string; score: number } | null = null;
  for (const account of accounts) {
    if (
      account.id === activeId ||
      (active && (account.provider !== active.provider || shape(account) !== shape(active)))
    )
      continue;
    const s = score(account);
    if (s !== null && s > 0 && (best === null || s > best.score)) best = { id: account.id, score: s };
  }
  return best && best.score > activeScore + 0.05 ? best.id : null;
}

function shape(account: Account): string {
  if (account.quota.status !== "ready") return "";
  return account.quota.groups
    .filter((g) => ["claude", "gemini", "codex"].includes(g.key))
    .map(
      (g) =>
        `${g.key}:${[g.fiveHour, g.weekly, ...g.extra]
          .filter((w): w is QuotaWindow => w !== null)
          .map((w) => w.durationMinutes ?? w.label)
          .sort()
          .join(",")}`,
    )
    .sort()
    .join("|");
}
