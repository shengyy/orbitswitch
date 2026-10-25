import { t } from "../i18n";
import { providerInfo } from "../providers";
import type { ProviderStatus } from "../types";
import { PlayIcon, PlusIcon, PowerIcon, RestartIcon } from "./icons";
import "./footer.css";

interface Props {
  status: ProviderStatus;
  onImport: () => void;
  busy: boolean;
  onAdd: () => void;
  onRestart: () => void;
  onStop: () => void;
}

export function Footer({ status, busy, onAdd, onImport, onRestart, onStop }: Props) {
  const { installed, running } = status;
  const vars = { client: providerInfo[status.provider].client };
  return (
    <footer className="panel-footer">
      <button type="button" className="footer-btn footer-add" disabled={busy} onClick={onAdd}>
        <PlusIcon />
        {t("footer.add")}
      </button>
      {status.provider !== "antigravity" && (
        <button
          type="button"
          className="footer-btn"
          disabled={busy}
          onClick={onImport}
          title={t("footer.importHint")}
        >
          {t("footer.import")}
        </button>
      )}
      <span className="footer-spacer" />
      {installed && running && (
        <>
          <button
            type="button"
            className="footer-btn"
            title={t("footer.restartTitle", vars)}
            disabled={busy || !status.desktopInstalled}
            onClick={onRestart}
          >
            <RestartIcon />
            {t("footer.restart")}
          </button>
          <button
            type="button"
            className="footer-btn"
            title={t("footer.stopTitle", vars)}
            disabled={busy}
            onClick={onStop}
          >
            <PowerIcon />
            {t("footer.stop")}
          </button>
        </>
      )}
      {status.desktopInstalled && !running && (
        <button type="button" className="footer-btn" disabled={busy} onClick={onRestart}>
          <PlayIcon />
          {t("footer.launch", vars)}
        </button>
      )}
    </footer>
  );
}
