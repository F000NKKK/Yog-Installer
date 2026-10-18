import { CheckField, useT } from "substrate-platform-ui";

/** Where things go: Yog's home (moved with YOG_HOME), its download cache, whether downloads are kept. */
export function LocationsTab({ home, cache, keepDownloads, onKeepDownloads }: { home: string; cache: string; keepDownloads: boolean; onKeepDownloads: (keep: boolean) => void }) {
  const t = useT();
  return (
    <div className="yi-locations">
      <section>
        <h3>{t("Yog home")}</h3>
        <p className="yi-muted">{t("SDKs, JDKs, products and tools go here. Set the YOG_HOME variable to move it.")}</p>
        <code className="yi-path">{home}</code>
      </section>
      <section>
        <h3>{t("Download cache")}</h3>
        <code className="yi-path">{cache}</code>
        <CheckField checked={keepDownloads} onChange={onKeepDownloads} label={t("Keep download cache after the installation")} />
      </section>
      <section>
        <h3>{t("Shared components, tools and SDKs")}</h3>
        <p className="yi-muted">{t("The Rust toolchain, its targets and cargo tools go where rustup and cargo keep them (~/.cargo).")}</p>
      </section>
    </div>
  );
}
