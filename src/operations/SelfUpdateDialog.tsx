import { useEffect, useState } from "react";
import { Button, ProgressBar, notify, useT } from "substrate-platform-ui";
import { api, type SelfUpdateProgress } from "../api";
import { formatBytes } from "../format";

/**
 * "The Installer needs to be updated", as Visual Studio Installer asks it
 * before anything else: Update downloads the new version here, with its
 * progress, and the Installer starts again by itself. Not now leaves it for
 * the banner.
 */
export function SelfUpdateDialog({ current, next, onLater }: { current: string; next: string; onLater: () => void }) {
  const t = useT();
  const [progress, setProgress] = useState<SelfUpdateProgress | null>(null);
  const [updating, setUpdating] = useState(false);

  useEffect(() => {
    const unlisten = api.onSelfUpdate(setProgress);
    return () => {
      void unlisten.then((u) => u());
    };
  }, []);

  function update() {
    setUpdating(true);
    api.selfUpdate().catch((e) => {
      setUpdating(false);
      setProgress(null);
      notify({ level: "error", message: String(e) });
    });
  }

  const fraction = progress?.total ? progress.done / progress.total : null;
  const done = progress !== null && progress.total !== null && progress.done >= progress.total;
  return (
    <div className="sp-dialog-backdrop">
      <div className="sp-dialog yi-self-update-dialog" role="dialog" aria-modal="true" aria-label={t("Yog Installer needs to be updated")}>
        <div className="sp-dialog-title">{t("Yog Installer needs to be updated")}</div>
        <div className="sp-dialog-message">
          {t("Before you go on, Yog Installer updates itself from {current} to {next}. It starts again by itself when it's done.", { current, next })}
        </div>
        {updating && (
          <div className="yi-self-update-progress">
            <div className="yi-op-row">
              <span>
                {done
                  ? t("Starting the new version…")
                  : progress
                    ? t("Downloading: {done} of {total}", { done: formatBytes(progress.done), total: progress.total !== null ? formatBytes(progress.total) : "?" })
                    : t("Updating…")}
              </span>
              <span>{fraction !== null ? `${Math.round(fraction * 100)}%` : ""}</span>
            </div>
            <ProgressBar value={done ? null : fraction} label={t("Downloading")} />
          </div>
        )}
        <div className="sp-dialog-buttons">
          <Button variant="primary" disabled={updating} onClick={update}>
            {t("Update")}
          </Button>
          <Button disabled={updating} onClick={onLater}>
            {t("Not now")}
          </Button>
        </div>
      </div>
    </div>
  );
}
