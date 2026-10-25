import { clock, percent, untilReset } from "../format";
import { t } from "../i18n";
import { severity } from "../quota";
import type { QuotaGroup, QuotaWindow } from "../types";
import "./quota-meter.css";

function Meter({ label, window }: { label: string; window: QuotaWindow | null }) {
  if (!window) {
    return (
      <div className="meter meter-empty">
        <span className="meter-label">{label}</span>
        <span className="meter-track" />
        <span className="meter-value">–</span>
      </div>
    );
  }
  const resetsIn = untilReset(window.resetAt);
  const tooltip =
    resetsIn && window.resetAt
      ? `${t("quota.resetsAt", { time: clock(window.resetAt) })} · ${t("quota.resetsIn", { time: resetsIn })}`
      : t("quota.unknownReset");
  return (
    <div className={`meter meter-${severity(window.remaining)}`} title={tooltip}>
      <span className="meter-label">{label}</span>
      <span className="meter-track">
        <span className="meter-fill" style={{ width: `${window.remaining * 100}%` }} />
      </span>
      <span className="meter-value">{percent(window.remaining)}</span>
    </div>
  );
}

/** One model group, displaying only the windows reported by its provider. */
export function QuotaRow({ group }: { group: QuotaGroup }) {
  return (
    <div
      className="quota-row"
      title={group.models.length ? t("quota.models", { list: group.models.join(", ") }) : undefined}
    >
      <span className="quota-group">{group.label}</span>
      <div className="quota-windows">
        {group.fiveHour && <Meter label={t("quota.fiveHour")} window={group.fiveHour} />}
        {group.weekly && <Meter label={t("quota.weekly")} window={group.weekly} />}
        {group.extra.map((window) => (
          <Meter key={`${window.label}-${window.durationMinutes}`} label={window.label} window={window} />
        ))}
      </div>
    </div>
  );
}
