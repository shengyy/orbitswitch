import { t } from "../i18n";
import { OrbitMark, PlusIcon } from "./icons";
import "./empty-state.css";

export function EmptyState({
  disabled,
  onAdd,
  client,
}: {
  disabled: boolean;
  onAdd: () => void;
  client: string;
}) {
  return (
    <div className="empty">
      <OrbitMark width={44} height={44} className="empty-mark" />
      <h2>{t("empty.title")}</h2>
      <p>{t("empty.body", { client })}</p>
      <button type="button" className="btn btn-primary" disabled={disabled} onClick={onAdd}>
        <PlusIcon />
        {t("footer.add")}
      </button>
    </div>
  );
}
