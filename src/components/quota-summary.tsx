import { percent } from "../format";
import { t } from "../i18n";
import { calculateAggregateQuota, severity } from "../quota";
import type { Account } from "../types";
import "./quota-summary.css";

export function ProviderQuotaSummary({ accounts }: { accounts: Account[] }) {
  const summary = calculateAggregateQuota(accounts);
  if (summary.accountCount === 0) return null;

  if (summary.fiveHour === null && summary.weekly === null && summary.readyCount === 0) {
    return (
      <div className="provider-quota-summary" aria-busy="true">
        <div className="meter meter-loading">
          <span className="meter-label">{t("quota.fiveHourTotal")}</span>
          <span className="meter-track" />
          <span className="meter-value">–</span>
        </div>
        <div className="meter meter-loading">
          <span className="meter-label">{t("quota.weeklyTotal")}</span>
          <span className="meter-track" />
          <span className="meter-value">–</span>
        </div>
      </div>
    );
  }

  const tip5h =
    summary.fiveHour !== null
      ? `${t("quota.fiveHourTotal")}: ${percent(summary.fiveHour)} · ${t("quota.accountCount", { count: summary.accountCount })}`
      : t("quota.unavailable");

  const tipWeekly =
    summary.weekly !== null
      ? `${t("quota.weeklyTotal")}: ${percent(summary.weekly)} · ${t("quota.accountCount", { count: summary.accountCount })}`
      : t("quota.unavailable");

  return (
    <div className="provider-quota-summary">
      <div
        className={`meter ${summary.fiveHour !== null ? `meter-${severity(summary.fiveHour)}` : "meter-empty"}`}
        title={tip5h}
      >
        <span className="meter-label">{t("quota.fiveHourTotal")}</span>
        <span className="meter-track">
          {summary.fiveHour !== null && (
            <span className="meter-fill" style={{ width: `${Math.round(summary.fiveHour * 100)}%` }} />
          )}
        </span>
        <span className="meter-value">{summary.fiveHour !== null ? percent(summary.fiveHour) : "–"}</span>
      </div>

      <div
        className={`meter ${summary.weekly !== null ? `meter-${severity(summary.weekly)}` : "meter-empty"}`}
        title={tipWeekly}
      >
        <span className="meter-label">{t("quota.weeklyTotal")}</span>
        <span className="meter-track">
          {summary.weekly !== null && (
            <span className="meter-fill" style={{ width: `${Math.round(summary.weekly * 100)}%` }} />
          )}
        </span>
        <span className="meter-value">{summary.weekly !== null ? percent(summary.weekly) : "–"}</span>
      </div>
    </div>
  );
}
