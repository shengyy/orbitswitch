import type { MouseEvent } from "react";
import { ago, displayName, untilReset } from "../format";
import { errorText, t } from "../i18n";
import type { Account, QuotaGroup } from "../types";
import { Avatar } from "./avatar";
import { MoreIcon, WarningIcon } from "./icons";
import { PlanBadge } from "./plan-badge";
import { QuotaRow } from "./quota-meter";
import "./account-card.css";

interface Props {
  account: Account;
  active: boolean;
  best: boolean;
  disabled: boolean;
  needsDesktop: boolean;
  onBind: () => void;
  onSwitch: () => void;
  onReauth: () => void;
  onMenu: (event: MouseEvent) => void;
}

/** The tightest low window that will recover, e.g. "Claude 5h · resets in 2h 14m". */
function recoveryHint(groups: QuotaGroup[]): string | null {
  const candidates = groups.flatMap((group) =>
    [
      { group, label: t("quota.fiveHour"), window: group.fiveHour },
      { group, label: t("quota.weekly"), window: group.weekly },
    ].filter((c) => c.window && c.window.remaining < 0.35),
  );
  candidates.sort((a, b) => (a.window?.remaining ?? 1) - (b.window?.remaining ?? 1));
  for (const { group, label, window } of candidates) {
    const time = untilReset(window?.resetAt ?? null);
    if (time) return `${group.label} ${label} · ${t("quota.resetsIn", { time })}`;
  }
  return null;
}

/** The earliest weekly quota reset duration for this account. */
function weeklyResetTime(account: Account): string | null {
  if (account.quota.status !== "ready") return null;
  let minResetAt: string | null = null;
  let minMs = Infinity;
  const now = Date.now();
  for (const group of account.quota.groups) {
    if (group.weekly?.resetAt) {
      const ms = new Date(group.weekly.resetAt).getTime() - now;
      if (ms > 0 && ms < minMs) {
        minMs = ms;
        minResetAt = group.weekly.resetAt;
      }
    }
  }
  return untilReset(minResetAt);
}

function QuotaBody({ account, onReauth }: Pick<Props, "account" | "onReauth">) {
  const { quota } = account;
  switch (quota.status) {
    case "loading":
      return (
        <div className="card-quota" aria-busy="true">
          <div className="skeleton" />
          <div className="skeleton" />
        </div>
      );
    case "reauth":
      return (
        <div className="card-notice">
          <WarningIcon />
          <span>{t("account.reauth")}</span>
          <button type="button" className="link-btn" onClick={onReauth}>
            {t("account.reauthAction")}
          </button>
        </div>
      );
    case "failed":
      return (
        <div className="card-notice" title={quota.message}>
          <WarningIcon />
          <span>{errorText(quota.code, quota.message)}</span>
        </div>
      );
    case "ready": {
      const groups = quota.groups;
      const hint = recoveryHint(groups);
      return (
        <div className="card-quota">
          {groups.map((group) => (
            <QuotaRow key={group.key + group.label} group={group} />
          ))}
          {!groups.length && <div className="card-hint">{t("quota.unavailable")}</div>}
          {quota.stale && (
            <div className="card-stale">{t("quota.stale", { time: ago(quota.fetchedAt) })}</div>
          )}
          {hint && <div className="card-hint">{hint}</div>}
        </div>
      );
    }
  }
}

export function AccountCard({
  account,
  active,
  best,
  disabled,
  needsDesktop,
  onBind,
  onSwitch,
  onReauth,
  onMenu,
}: Props) {
  const name = displayName(account.name, account.email);
  const canSwitch = !active && !disabled && !needsDesktop && account.quota.status !== "reauth";
  const weeklyReset = weeklyResetTime(account);
  const classes = ["card", active && "card-active", canSwitch && "card-switchable"].filter(Boolean).join(" ");
  return (
    // biome-ignore lint/a11y/useKeyWithClickEvents: the account identity button is the keyboard path.
    <article
      className={classes}
      onClick={canSwitch ? onSwitch : undefined}
      onContextMenu={(event) => {
        event.preventDefault();
        onMenu(event);
      }}
    >
      {active && (
        <svg className="card-marching-ants" aria-hidden="true">
          <rect x="1" y="1" width="calc(100% - 2px)" height="calc(100% - 2px)" rx="11" ry="11" />
        </svg>
      )}
      <header className="card-head">
        <button
          type="button"
          className="card-identity"
          aria-label={[
            name,
            account.plan?.kind.toUpperCase(),
            active && t("account.current"),
            !active && best && t("account.best"),
            canSwitch ? t("account.switchTo", { email: account.email }) : account.email,
          ]
            .filter(Boolean)
            .join(" · ")}
          disabled={!canSwitch}
          onClick={(event) => {
            event.stopPropagation();
            onSwitch();
          }}
        >
          <Avatar id={account.id} label={name} picture={account.picture} />
          <span className="card-name">
            <span className="truncate">{name}</span>
            <PlanBadge plan={account.plan} />
          </span>
          <span className="card-email truncate">{account.email}</span>
        </button>
        <div className="card-head-end">
          <div className="card-head-row-top">
            {weeklyReset && (
              <span className="card-weekly-reset">{t("quota.weekResetsIn", { time: weeklyReset })}</span>
            )}
            <button
              type="button"
              className="icon-btn card-more"
              aria-label={t("account.actions")}
              onClick={(event) => {
                event.stopPropagation();
                onMenu(event);
              }}
            >
              <MoreIcon />
            </button>
          </div>
          <div className="card-head-row-bottom">
            {active && <span className="tag tag-current">{t("account.current")}</span>}
            {!active && best && <span className="tag tag-best">{t("account.best")}</span>}
          </div>
        </div>
      </header>
      <QuotaBody account={account} onReauth={onReauth} />
      {needsDesktop && (
        <div className="card-binding">
          {t("bind.needed")}
          <button
            type="button"
            className="link-btn"
            disabled={disabled}
            onClick={(event) => {
              event.stopPropagation();
              onBind();
            }}
          >
            {t("bind.action")}
          </button>
        </div>
      )}
    </article>
  );
}
