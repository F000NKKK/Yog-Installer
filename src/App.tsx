import { useCallback, useEffect, useRef, useState } from "react";
import { Button, DialogHost, NotificationHost, Tab, confirmDialog, notify, useLocale, useT } from "substrate-platform-ui";
import { api, type Product, type Selection, type Snapshot } from "./api";
import { ModifyWindow, type ModifySettings } from "./modify/ModifyWindow";
import { NewsPanel } from "./home/NewsPanel";
import { ProductCard } from "./home/ProductCard";
import { SelfUpdateDialog } from "./operations/SelfUpdateDialog";
import { TitleBar } from "./TitleBar";
import { usePersisted } from "./usePersisted";
import { useOperation } from "./operations/useOperation";
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
  const [selfUpdateLater, setSelfUpdateLater] = useState(false);
  const [checking, setChecking] = useState(false);

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
  const locale = useLocale();

  useEffect(() => load(false), [load]);

  // An install or modify that didn't end goes on by itself, once a start.
  const resumed = useRef(false);
  useEffect(() => {
    if (!snapshot?.pending || resumed.current) return;
    resumed.current = true;
    setPage("installed");
    notify({ message: t("Going on with the installation that didn't finish.") });
    api.resume().catch((e) => notify({ level: "error", message: String(e) }));
  }, [snapshot, t]);

  const installed = snapshot?.products.filter((p) => p.installed !== null) ?? [];
  const available = snapshot?.products.filter((p) => p.installed === null) ?? [];
  const shown = page === "installed" ? installed : available;
  const busy = operation.running;

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

  /** Asks every source again and says what came of it. */
  async function checkForUpdates() {
    setChecking(true);
    try {
      const s = await api.snapshot(true);
      setSnapshot(s);
      if (s.installerUpdate) setSelfUpdateLater(false);
      const count = s.products.filter((p) => p.updateAvailable).length + (s.installerUpdate ? 1 : 0);
      notify({ message: count > 0 ? t("Updates available: {count}.", { count }) : t("Everything is up to date.") });
    } catch (e) {
      notify({ level: "error", message: String(e) });
    } finally {
      setChecking(false);
    }
  }

  return (
    <div className="yi-app">
      <TitleBar notifications={snapshot?.problems ?? []} />
      {snapshot?.installerUpdate && selfUpdateLater && (
        <div className="yi-self-update">
          <span>{t("A newer Yog Installer is out ({version}). It updates itself before anything else.", { version: snapshot.installerUpdate })}</span>
          <Button variant="primary" disabled={busy} onClick={() => setSelfUpdateLater(false)}>
            {t("Update")}
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
        <Button variant="ghost" disabled={busy || checking} onClick={() => void checkForUpdates()}>
          {checking ? t("Checking for updates…") : t("Check for updates")}
        </Button>
      </nav>
      <div className="yi-body">
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
      <NewsPanel news={snapshot?.news ?? []} locale={locale} />
      </div>
      <footer className="yi-statusbar">{snapshot && t("Installer version {version}", { version: snapshot.version })}</footer>
      {modifying && snapshot && (
        <ModifyWindow product={modifying} snapshot={snapshot} settings={settings} onSettings={setSettings} onApply={(s) => apply(modifying, s)} onClose={() => setModifying(null)} />
      )}
      {snapshot?.installerUpdate && !selfUpdateLater && !busy && (
        <SelfUpdateDialog current={snapshot.version} next={snapshot.installerUpdate} onLater={() => setSelfUpdateLater(true)} />
      )}
      <DialogHost />
      <NotificationHost />
    </div>
  );
}
