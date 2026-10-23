import { useEffect, useRef, useState } from "react";
import { Icon, IconButton, useT } from "substrate-platform-ui";

/**
 * The title bar's bell, as Visual Studio Installer's: its count is what
 * hasn't been seen; opening it shows them all and they count as seen; each
 * can be dismissed.
 */
export function Notifications({ messages }: { messages: string[] }) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const [seen, setSeen] = useState<Set<string>>(new Set());
  const [dismissed, setDismissed] = useState<Set<string>>(new Set());
  const root = useRef<HTMLSpanElement>(null);
  const shown = messages.filter((m) => !dismissed.has(m));
  const unseen = shown.filter((m) => !seen.has(m)).length;

  useEffect(() => {
    if (!open) return;
    setSeen((s) => new Set([...s, ...shown]));
    const close = (e: MouseEvent) => !root.current?.contains(e.target as Node) && setOpen(false);
    window.addEventListener("mousedown", close);
    return () => window.removeEventListener("mousedown", close);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, messages]);

  return (
    <span className="yi-bell" ref={root}>
      <IconButton title={t("Notifications")} onClick={() => setOpen((o) => !o)}>
        <Icon name="bell" />
      </IconButton>
      {unseen > 0 && <span className="yi-badge">{unseen}</span>}
      {open && (
        <div className="yi-flyout" role="dialog" aria-label={t("Notifications")}>
          <h3>{t("Notifications")}</h3>
          {shown.length === 0 && <p className="yi-muted">{t("No notifications.")}</p>}
          {shown.map((m) => (
            <div key={m} className="yi-flyout-item">
              <Icon name="warning" size={14} className="yi-flyout-icon" />
              <span>{m}</span>
              <IconButton title={t("Dismiss")} onClick={() => setDismissed((d) => new Set([...d, m]))}>
                <Icon name="close" size={12} />
              </IconButton>
            </div>
          ))}
        </div>
      )}
    </span>
  );
}
