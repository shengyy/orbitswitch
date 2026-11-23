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
    const metrics = getWeeklyMetrics(account);
    if (metrics.hasWeekly && metrics.minRemaining < 0.05) continue;
    const s = score(account);
    if (s !== null && s > 0 && (best === null || s > best.score)) best = { id: account.id, score: s };
  }
  return best && best.score > activeScore + 0.05 ? best.id : null;
}

export interface WeeklyMetrics {
  hasWeekly: boolean;
  minRemaining: number;
  earliestResetMs: number;
}

export function getWeeklyMetrics(account: Account, now = Date.now()): WeeklyMetrics {
  if (account.quota.status !== "ready") {
    return { hasWeekly: false, minRemaining: 1, earliestResetMs: Infinity };
  }
  let hasWeekly = false;
  let minRemaining = 1;
  let earliestResetMs = Infinity;

  for (const group of account.quota.groups) {
    if (group.weekly) {
      hasWeekly = true;
      if (group.weekly.remaining < minRemaining) {
        minRemaining = group.weekly.remaining;
      }
      if (group.weekly.resetAt) {
        const ms = new Date(group.weekly.resetAt).getTime() - now;
        if (ms > 0 && ms < earliestResetMs) {
          earliestResetMs = ms;
        }
      }
    }
  }

  return { hasWeekly, minRemaining, earliestResetMs };
}

/**
 * Sorts accounts:
 * 1. The active account leads the list.
 * 2. Accounts with weekly quota remaining < 5% are deprioritized to the very end.
 * 3. Eligible accounts are sorted by earliest weekly reset time ascending (use soon-to-reset first).
 * 4. Ties break by larger remaining weekly share.
 */
export function sortAccounts(accounts: Account[], activeId: string | null, now = Date.now()): Account[] {
  return [...accounts].sort((a, b) => {
    const aActive = a.id === activeId;
    const bActive = b.id === activeId;
    if (aActive !== bActive) return aActive ? -1 : 1;

    const aMetrics = getWeeklyMetrics(a, now);
    const bMetrics = getWeeklyMetrics(b, now);

    const aExhausted = aMetrics.hasWeekly && aMetrics.minRemaining < 0.05;
    const bExhausted = bMetrics.hasWeekly && bMetrics.minRemaining < 0.05;
    if (aExhausted !== bExhausted) return aExhausted ? 1 : -1;

    if (aMetrics.earliestResetMs !== bMetrics.earliestResetMs) {
      return aMetrics.earliestResetMs - bMetrics.earliestResetMs;
    }

    if (aMetrics.minRemaining !== bMetrics.minRemaining) {
      return bMetrics.minRemaining - aMetrics.minRemaining;
    }

    return 0;
  });
}

export interface AggregateQuota {
  fiveHour: number | null;
  weekly: number | null;
  accountCount: number;
  readyCount: number;
}

/**
 * Calculates aggregate 5H and weekly remaining fractions across accounts.
 *
 * NOTE: An account's 5H quota is bottlenecked by its weekly capacity.
 * A 5-hour window represents approximately 20% of weekly quota (5 sessions/week).
 * For example, if an account only has 1% weekly quota left, its maximum effective
 * 5H quota is capped at 1% * 5 = 5%, even if the 5H rolling window is at 100%.
 */
export function calculateAggregateQuota(accounts: Account[]): AggregateQuota {
  if (accounts.length === 0) {
    return { fiveHour: null, weekly: null, accountCount: 0, readyCount: 0 };
  }

  let total5h = 0;
  let totalWeekly = 0;
  let valid5hCount = 0;
  let validWeeklyCount = 0;
  let readyCount = 0;

  for (const account of accounts) {
    if (account.quota.status === "ready") {
      readyCount++;
      const groups = account.quota.groups;
      if (groups.length === 0) continue;

      let accountMinWeekly: number | null = null;
      let accountMin5h: number | null = null;

      for (const group of groups) {
        if (group.weekly) {
          accountMinWeekly =
            accountMinWeekly === null
              ? group.weekly.remaining
              : Math.min(accountMinWeekly, group.weekly.remaining);
        }
        if (group.fiveHour) {
          accountMin5h =
            accountMin5h === null
              ? group.fiveHour.remaining
              : Math.min(accountMin5h, group.fiveHour.remaining);
        }
      }

      let effective5h: number | null = null;
      if (accountMin5h !== null) {
        effective5h =
          accountMinWeekly !== null ? Math.min(accountMin5h, accountMinWeekly * 5.0) : accountMin5h;
      } else if (accountMinWeekly !== null) {
        effective5h = Math.min(1.0, accountMinWeekly * 5.0);
      }

      if (effective5h !== null) {
        total5h += Math.max(0, Math.min(1.0, effective5h));
        valid5hCount++;
      }

      if (accountMinWeekly !== null) {
        totalWeekly += Math.max(0, Math.min(1.0, accountMinWeekly));
        validWeeklyCount++;
      }
    } else if (account.quota.status === "reauth" || account.quota.status === "failed") {
      total5h += 0;
      valid5hCount++;
      totalWeekly += 0;
      validWeeklyCount++;
    }
  }

  return {
    fiveHour: valid5hCount > 0 ? total5h / valid5hCount : null,
    weekly: validWeeklyCount > 0 ? totalWeekly / validWeeklyCount : null,
    accountCount: accounts.length,
    readyCount,
  };
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
