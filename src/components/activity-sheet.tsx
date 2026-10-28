import { useState } from "react";
import { displayName } from "../format";
import { t } from "../i18n";
import { providerInfo } from "../providers";
import type { Account, Operation, SwitchStep } from "../types";
import { CheckIcon, OrbitMark } from "./icons";
import { Sheet, SheetActions } from "./sheet";
import "./activity-sheet.css";

const STEPS: SwitchStep[] = ["preparing", "closing", "writing", "launching"];

interface Props {
  operation: Exclude<Operation, { kind: "idle" }>;
  accounts: Account[];
  onCancelAdd: () => void;
  onCopySignInLink: () => Promise<void>;
}

/** Progress for whatever the backend is doing right now. */
export function ActivitySheet({ operation, accounts, onCancelAdd, onCopySignInLink }: Props) {
  const [copyState, setCopyState] = useState<"idle" | "copying" | "copied" | "failed">("idle");
  const vars = "provider" in operation ? providerInfo[operation.provider] : {};
  if (operation.kind === "adding") {
    return (
      <Sheet onDismiss={onCancelAdd}>
        <div className="activity-hero">
          <OrbitMark className="orbiting" width={40} height={40} />
        </div>
        <h2 className="center">{t("adding.title")}</h2>
        <p className="center">{t("adding.body", vars)}</p>
        {operation.provider === "antigravity" && (
          <p className="sheet-note center" role="status">
            {t(copyState === "failed" ? "adding.copyFailed" : "adding.privateHint")}
          </p>
        )}
        <SheetActions>
          <button type="button" className="btn wide" onClick={onCancelAdd}>
            {t("adding.cancel")}
          </button>
          {operation.provider === "antigravity" && (
            <button
              type="button"
              className="btn btn-primary wide"
              disabled={!operation.canCopyLink || copyState === "copying"}
              onClick={async () => {
                setCopyState("copying");
                try {
                  await onCopySignInLink();
                  setCopyState("copied");
                } catch {
                  setCopyState("failed");
                }
              }}
            >
              {t(copyState === "copied" ? "adding.copied" : "adding.copyLink")}
            </button>
          )}
        </SheetActions>
      </Sheet>
    );
  }

  if (operation.kind === "switching") {
    const target = accounts.find((a) => a.id === operation.targetId);
    const current = STEPS.indexOf(operation.step);
    return (
      <Sheet>
        <h2>{target ? displayName(target.name, target.email) : ""}</h2>
        <ol className="steps">
          {STEPS.map((step, index) => (
            <li
              key={step}
              className={index < current ? "step done" : index === current ? "step now" : "step"}
            >
              <span className="step-mark">
                {index < current ? <CheckIcon /> : <span className="step-dot" />}
              </span>
              {t(`progress.${step}`, vars)}
            </li>
          ))}
        </ol>
      </Sheet>
    );
  }

  if (operation.kind === "updating") {
    const percent = operation.total
      ? Math.min(100, Math.floor((operation.downloaded / operation.total) * 100))
      : null;
    return (
      <Sheet>
        <h2>{t("update.progressTitle")}</h2>
        <div className="activity-line">
          <span className="spinner" />
          {t(`update.${operation.step}`)}
          {operation.step === "downloading" && percent !== null && <span>{percent}%</span>}
        </div>
        {operation.step === "downloading" && (
          <progress className="update-progress" max={100} value={percent ?? undefined} />
        )}
        <p className="sheet-note">{t("update.body")}</p>
      </Sheet>
    );
  }

  return (
    <Sheet>
      <div className="activity-line">
        <span className="spinner" />
        {t(`progress.${operation.kind}`, vars)}
      </div>
    </Sheet>
  );
}
