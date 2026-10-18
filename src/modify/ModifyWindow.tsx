import { useState } from "react";
import { Button, IconButton, Icon, Tab, useT } from "substrate-platform-ui";
import type { Mode, Product, Selection, Snapshot } from "../api";
import { formatBytes } from "../format";
import { ComponentsTab } from "./ComponentsTab";
import { DetailsPanel } from "./DetailsPanel";
import { LanguagePacksTab } from "./LanguagePacksTab";
import { LocationsTab } from "./LocationsTab";
import { WorkloadsTab } from "./WorkloadsTab";
import { useModify } from "./useModify";
import "./Modify.css";

type Page = "workloads" | "components" | "languages" | "locations";

export interface ModifySettings {
  mode: Mode;
  keepDownloads: boolean;
}

/**
 * Installing or modifying a product, Visual Studio Installer's way:
 * Workloads, Individual components, Language packs, Installation locations;
 * Installation details on the right; the location, the space it takes, the
 * download mode and the button at the bottom.
 */
export function ModifyWindow({ product, snapshot, settings, onSettings, onApply, onClose }: {
  product: Product;
  snapshot: Snapshot;
  settings: ModifySettings;
  onSettings: (s: ModifySettings) => void;
  onApply: (selection: Selection) => void;
  onClose: () => void;
}) {
  const t = useT();
  const [page, setPage] = useState<Page>("workloads");
  const { selection, plan, error, toggleWorkload, setComponent } = useModify(product.id);
  const installing = product.installed === null;
  const ids = snapshot.productWorkloads.find(([id]) => id === product.id)?.[1] ?? [];
  const workloads = snapshot.workloads.filter((w) => ids.includes(w.id));
  const chosen = workloads.filter((w) => selection?.workloads.includes(w.id));
  const reasons = plan?.reasons ?? {};
  const nothingToDo = plan !== null && plan.changes.length === 0;
  const blocked = !selection || !plan || nothingToDo || plan.conflicts.length > 0;

  const tabs: [Page, string][] = [
    ["workloads", t("Workloads")],
    ["components", t("Individual components")],
    ["languages", t("Language packs")],
    ["locations", t("Installation locations")],
  ];

  return (
    <div className="yi-modify" role="dialog" aria-label={product.name}>
      <header className="yi-modify-header" data-tauri-drag-region>
        <span data-tauri-drag-region>
          {installing ? t("Installing") : t("Modifying")} — {product.name} — {product.installed ?? product.latest}
        </span>
        <IconButton title={t("Close")} onClick={onClose}>
          <Icon name="close" />
        </IconButton>
      </header>
      <nav className="yi-modify-tabs">
        {tabs.map(([id, label]) => (
          <Tab key={id} active={page === id} onClick={() => setPage(id)}>
            {label}
          </Tab>
        ))}
      </nav>
      <div className="yi-modify-body">
        <main className="yi-modify-page">
          {error && <p className="yi-op-error">{error}</p>}
          {selection && page === "workloads" && <WorkloadsTab workloads={workloads} chosen={selection.workloads} onToggle={toggleWorkload} />}
          {selection && page === "components" && <ComponentsTab components={snapshot.components} groups={snapshot.groups} reasons={reasons} onSet={setComponent} />}
          {page === "languages" && <LanguagePacksTab />}
          {page === "locations" && (
            <LocationsTab home={snapshot.home} cache={snapshot.cache} keepDownloads={settings.keepDownloads} onKeepDownloads={(keepDownloads) => onSettings({ ...settings, keepDownloads })} />
          )}
        </main>
        <DetailsPanel workloads={chosen} components={snapshot.components} reasons={reasons} onSet={setComponent} />
      </div>
      {plan && plan.conflicts.length > 0 && (
        <div className="yi-conflicts">
          {plan.conflicts.map((c) => (
            <div key={`${c.component}-${c.needs}`}>
              {t("{component} {version} needs {needs} {requirement} — {found}.", {
                component: c.component,
                version: c.version,
                needs: c.needs,
                requirement: c.requirement,
                found: c.found ? t("there will be {version}", { version: c.found }) : t("it won't be installed"),
              })}
            </div>
          ))}
        </div>
      )}
      <footer className="yi-modify-footer">
        <div className="yi-modify-location">
          <span className="yi-muted">{t("Location")}</span>
          <code className="yi-path">{snapshot.home}</code>
        </div>
        <div className="yi-modify-total">
          <span className="yi-muted">{t("Total download")}</span>
          <strong>{plan ? formatBytes(plan.downloadBytes) : "…"}</strong>
        </div>
        <select className="yi-mode" value={settings.mode} onChange={(e) => onSettings({ ...settings, mode: e.target.value as Mode })}>
          <option value="installWhileDownloading">{t("Install while downloading")}</option>
          <option value="downloadAllThenInstall">{t("Download all, then install")}</option>
        </select>
        <Button variant="primary" disabled={blocked} onClick={() => selection && onApply(selection)}>
          {installing ? t("Install") : t("Modify")}
        </Button>
      </footer>
    </div>
  );
}
