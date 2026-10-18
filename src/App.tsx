import { useCallback, useEffect, useState } from "react";
import { Button, DialogHost, NotificationHost, Tab, confirmDialog, notify, useT } from "substrate-platform-ui";
import { api, type Product, type Selection, type Snapshot } from "./api";
import { ModifyWindow, type ModifySettings } from "./modify/ModifyWindow";
import { ProductCard } from "./ProductCard";
import { TitleBar } from "./TitleBar";
import { usePersisted } from "./usePersisted";
import { useOperation } from "./useOperation";
import "./App.css";

type Page = "installed" | "available";

const DEFAULT_SETTINGS: ModifySettings = { mode: "installWhileDownloading", keepDownloads: false };

/** Yog Installer's window: the Installed and Available products, as Visual Studio Installer's. */
export default function App() {
  const t = useT();
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [page, setPage] = useState<Page>("installed");
  const [modifying, setModifying] = useState<Product | null>(null);
  const [settings, setSettings] = usePersisted<ModifySettings>("modify.settings", DEFAULT_SETTINGS);
  const [updatingSelf, setUpdatingSelf] = useState(false);

  const load = useCallback((refresh: boolean) => {
    api.snapshot(refresh).then(
      (s) => {
        setSnapshot(s);
        setError(null);
      },
      (e) => setError(String(e)),
    );
  }, []);
  const operation = useOperation(() => load(false));

  useEffect(() => load(false), [load]);

  const installed = snapshot?.products.filter((p) => p.installed !== null) ?? [];
  const available = snapshot?.products.filter((p) => p.installed === null) ?? [];
  const shown = page === "installed" ? installed : available;
  const busy = operation.running || updatingSelf;

  const guard = (promise: Promise<void>) => promise.catch((e) => notify({ level: "error", message: String(e) }));

  async function update(p: Product) {
    const selection = await api.selectionOf(p.id);
    await guard(api.apply(p.id, selection, settings.mode, settings.keepDownloads));
  }

  async function uninstall(p: Product) {
    const sure = await confirmDialog({ title: t("Uninstall"), message: t("Uninstall {product}? Its components stay: other products may use them.", { product: p.name }), confirmLabel: t("Uninstall") });
    if (sure) await guard(api.uninstall(p.id));
  }

  function apply(p: Product, selection: Selection) {
    setModifying(null);
    setPage("installed");
    void guard(api.apply(p.id, selection, settings.mode, settings.keepDownloads));
  }

  function selfUpdate() {
    setUpdatingSelf(true);
    api.selfUpdate().catch((e) => {
      setUpdatingSelf(false);
      notify({ level: "error", message: String(e) });
    });
  }

  function showProblems() {
    const problems = snapshot?.problems ?? [];
    if (problems.length === 0) notify({ message: t("No notifications.") });
    for (const message of problems) notify({ level: "warn", message, timeout: 0 });
  }

  return (
    <div className="yi-app">
      <TitleBar notifications={snapshot?.problems.length ?? 0} onNotifications={showProblems} />
      {snapshot?.installerUpdate && (
        <div className="yi-self-update">
          <span>{t("A newer Yog Installer is out ({version}). It updates itself before anything else.", { version: snapshot.installerUpdate })}</span>
          <Button variant="primary" disabled={busy} onClick={selfUpdate}>
            {updatingSelf ? t("Updating…") : t("Update")}
          </Button>
        </div>
      )}
      <nav className="yi-tabs">
        <Tab active={page === "installed"} onClick={() => setPage("installed")}>
          {t("Installed")}
        </Tab>
        <Tab active={page === "available"} onClick={() => setPage("available")}>
          {t("Available")}
        </Tab>
        <span className="yi-tabs-spacer" />
        <Button variant="ghost" disabled={busy} onClick={() => load(true)}>
          {t("Check for updates")}
        </Button>
      </nav>
      <main className="yi-products">
        {error && <p className="yi-op-error">{error}</p>}
        {!snapshot && !error && <p className="yi-muted">{t("Looking for what's installed and what's new…")}</p>}
        {snapshot && shown.length === 0 && (
          <p className="yi-muted">{page === "installed" ? t("Nothing is installed yet — see Available.") : t("Everything is installed.")}</p>
        )}
        {shown.map((p) => (
          <ProductCard
            key={p.id}
            product={p}
            busy={busy}
            op={operation.op?.product === p.id ? operation.op : null}
            actions={{
              onModify: () => setModifying(p),
              onLaunch: () => void guard(api.launch(p.id)),
              onUpdate: () => void update(p),
              onUninstall: () => void uninstall(p),
              onRepair: () => void guard(api.repair(p.id)),
              onPause: operation.pause,
              onCancel: operation.cancel,
              onDismiss: operation.dismiss,
            }}
          />
        ))}
      </main>
      <footer className="yi-statusbar">{snapshot && t("Installer version {version}", { version: snapshot.version })}</footer>
      {modifying && snapshot && (
        <ModifyWindow product={modifying} snapshot={snapshot} settings={settings} onSettings={setSettings} onApply={(s) => apply(modifying, s)} onClose={() => setModifying(null)} />
      )}
      <DialogHost />
      <NotificationHost />
    </div>
  );
}
