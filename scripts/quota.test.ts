import { describe, expect, test } from "bun:test";
import { available, bestAccountId, calculateAggregateQuota, sortAccounts } from "../src/quota";
import type { Account } from "../src/types";

function account(id: string, remaining: number, provider: Account["provider"] = "codex"): Account {
  return {
    id,
    provider,
    desktopReady: true,
    email: `${id}@example.com`,
    name: null,
    picture: null,
    plan: null,
    quota: {
      status: "ready",
      fetchedAt: new Date().toISOString(),
      stale: false,
      groups: [
        {
          key: provider === "codex" ? "codex" : "claude",
          label: "Coding",
          models: [],
          fiveHour: { remaining, resetAt: null, label: "5h", durationMinutes: 300 },
          weekly: null,
          extra: [],
        },
      ],
    },
  };
}
function weeklyAccount(id: string, remaining: number): Account {
  const result = account(id, remaining);
  if (result.quota.status === "ready") {
    result.quota.groups[0].fiveHour = null;
    result.quota.groups[0].weekly = {
      remaining,
      resetAt: null,
      label: "weekly",
      durationMinutes: 10080,
    };
  }
  return result;
}
describe("account recommendations", () => {
  test("weekly-only quota remains usable, including an exhausted week", () => {
    for (const remaining of [0, 0.4, 1]) {
      const a = weeklyAccount("weekly", remaining);
      if (a.quota.status === "ready") expect(available(a.quota.groups[0])).toBe(remaining);
    }
  });
  test("weekly-only accounts are comparable with each other", () => {
    expect(bestAccountId([weeklyAccount("active", 0.1), weeklyAccount("other", 0.8)], "active")).toBe(
      "other",
    );
  });
  test("weekly-only and dual-window accounts are not directly comparable", () => {
    const weekly = weeklyAccount("weekly", 0.1);
    const dual = weeklyAccount("dual", 0.8);
    if (dual.quota.status === "ready")
      dual.quota.groups[0].fiveHour = { remaining: 1, resetAt: null, label: "5h", durationMinutes: 300 };
    expect(bestAccountId([weekly, dual], "weekly")).toBeNull();
    if (dual.quota.status === "ready" && dual.quota.groups[0].fiveHour)
      dual.quota.groups[0].fiveHour.remaining = 0;
    expect(bestAccountId([weekly, dual], "dual")).toBeNull();
  });
  test("does not recommend a stale account", () => {
    const stale = account("other", 1);
    if (stale.quota.status === "ready") stale.quota.stale = true;
    expect(bestAccountId([account("active", 0.1), stale], "active")).toBeNull();
  });
  test("unknown active quota is not treated as exhausted", () => {
    const active = account("active", 0);
    active.quota = { status: "failed", code: "network", message: "fixture" };
    expect(bestAccountId([active, account("other", 1)], "active")).toBeNull();
  });
  test("never compares different providers", () => {
    expect(bestAccountId([account("active", 0), account("other", 1, "claude")], "active")).toBeNull();
  });
  test("different window durations are not comparable", () => {
    const other = account("other", 1);
    if (other.quota.status === "ready" && other.quota.groups[0].fiveHour)
      other.quota.groups[0].fiveHour.durationMinutes = 60;
    expect(bestAccountId([account("active", 0), other], "active")).toBeNull();
  });
  test("an Argon week does not affect Antigravity account recommendations", () => {
    const active = account("active", 0.1, "antigravity");
    const other = account("other", 0.8, "antigravity");
    if (active.quota.status === "ready") {
      active.quota.groups.push({
        key: "argon",
        label: "Argon",
        models: [],
        fiveHour: null,
        weekly: { remaining: 1, resetAt: null, label: "weekly", durationMinutes: 10080 },
        extra: [],
      });
    }
    expect(bestAccountId([active, other], active.id)).toBe(other.id);
  });
  test("a fresh compatible account with more remaining share is recommended", () => {
    expect(bestAccountId([account("active", 0), account("other", 0.8)], "active")).toBe("other");
  });

  test("sortAccounts prioritizes active account and soonest weekly reset, deprioritizing < 5%", () => {
    const now = 1700000000000;
    const a1 = weeklyAccount("a1", 0.8);
    const a2 = weeklyAccount("a2", 0.5);
    const a3 = weeklyAccount("a3", 0.03); // exhausted (< 5%)
    const aActive = weeklyAccount("active", 0.7);

    if (a1.quota.status === "ready" && a1.quota.groups[0].weekly) {
      a1.quota.groups[0].weekly.resetAt = new Date(now + 6 * 86400000).toISOString(); // 6 days
    }
    if (a2.quota.status === "ready" && a2.quota.groups[0].weekly) {
      a2.quota.groups[0].weekly.resetAt = new Date(now + 2 * 86400000).toISOString(); // 2 days (soonest)
    }
    if (a3.quota.status === "ready" && a3.quota.groups[0].weekly) {
      a3.quota.groups[0].weekly.resetAt = new Date(now + 1 * 86400000).toISOString(); // 1 day but < 5%
    }
    if (aActive.quota.status === "ready" && aActive.quota.groups[0].weekly) {
      aActive.quota.groups[0].weekly.resetAt = new Date(now + 7 * 86400000).toISOString();
    }

    const sorted = sortAccounts([a1, a3, aActive, a2], "active", now);
    expect(sorted.map((a) => a.id)).toEqual(["active", "a2", "a1", "a3"]);
  });

  test("calculateAggregateQuota clamps 5H quota to weekly * 5 when weekly is low (e.g. 1% week -> 5% 5h)", () => {
    // Account with 100% 5h, but only 1% week: effective 5h must be capped at 5%
    const acc1 = account("acc1", 1.0);
    if (acc1.quota.status === "ready") {
      acc1.quota.groups[0].weekly = {
        remaining: 0.01,
        resetAt: null,
        label: "weekly",
        durationMinutes: 10080,
      };
    }
    const res1 = calculateAggregateQuota([acc1]);
    expect(res1.weekly).toBeCloseTo(0.01);
    expect(res1.fiveHour).toBeCloseTo(0.05);

    // Account with 100% 5h, 20% week: effective 5h can reach 100%
    const acc2 = account("acc2", 1.0);
    if (acc2.quota.status === "ready") {
      acc2.quota.groups[0].weekly = {
        remaining: 0.2,
        resetAt: null,
        label: "weekly",
        durationMinutes: 10080,
      };
    }
    const res2 = calculateAggregateQuota([acc2]);
    expect(res2.weekly).toBeCloseTo(0.2);
    expect(res2.fiveHour).toBeCloseTo(1.0);

    // Account with 80% 5h, 10% week: effective 5h is min(0.8, 0.1 * 5) = 50%
    const acc3 = account("acc3", 0.8);
    if (acc3.quota.status === "ready") {
      acc3.quota.groups[0].weekly = {
        remaining: 0.1,
        resetAt: null,
        label: "weekly",
        durationMinutes: 10080,
      };
    }
    const res3 = calculateAggregateQuota([acc3]);
    expect(res3.weekly).toBeCloseTo(0.1);
    expect(res3.fiveHour).toBeCloseTo(0.5);
  });

  test("calculateAggregateQuota computes mean across multiple accounts including failed as 0", () => {
    const acc1 = account("acc1", 1.0);
    if (acc1.quota.status === "ready") {
      acc1.quota.groups[0].weekly = {
        remaining: 1.0,
        resetAt: null,
        label: "weekly",
        durationMinutes: 10080,
      };
    }
    const acc2 = account("acc2", 0);
    acc2.quota = { status: "reauth" };

    const res = calculateAggregateQuota([acc1, acc2]);
    expect(res.accountCount).toBe(2);
    expect(res.fiveHour).toBeCloseTo(0.5);
    expect(res.weekly).toBeCloseTo(0.5);
  });
});
