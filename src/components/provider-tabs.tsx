import { t } from "../i18n";
import { PROVIDERS, providerInfo } from "../providers";
import type { Provider } from "../types";
import "./provider-tabs.css";
export function ProviderTabs({
  value,
  onChange,
}: {
  value: Provider;
  onChange: (provider: Provider) => void;
}) {
  return (
    <div className="provider-tabs" role="tablist" aria-label={t("tabs.label")}>
      {PROVIDERS.map((provider, index) => (
        <button
          key={provider}
          id={`tab-${provider}`}
          type="button"
          role="tab"
          aria-selected={provider === value}
          aria-controls="accounts-panel"
          tabIndex={provider === value ? 0 : -1}
          onClick={() => onChange(provider)}
          onKeyDown={(event) => {
            const next =
              event.key === "ArrowRight"
                ? (index + 1) % 3
                : event.key === "ArrowLeft"
                  ? (index + 2) % 3
                  : event.key === "Home"
                    ? 0
                    : event.key === "End"
                      ? 2
                      : null;
            if (next === null) return;
            event.preventDefault();
            onChange(PROVIDERS[next]);
            document.getElementById(`tab-${PROVIDERS[next]}`)?.focus();
          }}
        >
          <span>{providerInfo[provider].label}</span>
          <small>{provider === "claude" ? "Claude Code" : providerInfo[provider].client}</small>
        </button>
      ))}
    </div>
  );
}
