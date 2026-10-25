import { describe, expect, test } from "bun:test";
import { bestAccountId } from "../src/quota";
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
describe("account recommendations", () => {
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
  test("a fresh compatible account with more remaining share is recommended", () => {
    expect(bestAccountId([account("active", 0), account("other", 0.8)], "active")).toBe("other");
  });
});
