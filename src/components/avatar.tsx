import { type CSSProperties, useState } from "react";
import "./avatar.css";

interface Props {
  id: string;
  label: string;
  picture: string | null;
}

/** Profile photo, or initials on a hue derived from the account id. */
export function Avatar({ id, label, picture }: Props) {
  const [failed, setFailed] = useState(false);
  if (picture && !failed) {
    return (
      <img
        className="avatar"
        src={picture}
        alt=""
        referrerPolicy="no-referrer"
        draggable={false}
        onError={() => setFailed(true)}
      />
    );
  }
  const hue = [...id].reduce((sum, ch) => (sum * 31 + ch.charCodeAt(0)) % 360, 7);
  return (
    <span className="avatar avatar-initials" style={{ "--hue": hue } as CSSProperties} aria-hidden="true">
      {label.trim().charAt(0).toUpperCase() || "?"}
    </span>
  );
}
