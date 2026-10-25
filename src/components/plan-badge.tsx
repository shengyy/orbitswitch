import type { Plan } from "../types";
import "./plan-badge.css";

export function PlanBadge({ plan }: { plan: Plan | null }) {
  if (!plan) return null;
  return (
    <span className={`plan-badge plan-${plan.kind}`} title={plan.name}>
      {plan.kind === "unknown" ? plan.name : plan.kind.toUpperCase()}
    </span>
  );
}
