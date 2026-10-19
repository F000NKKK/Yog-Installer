import { Button, ContextMenu, Icon, useContextMenu, openLink, useT } from "substrate-platform-ui";
import type { Product } from "./api";
import { formatBytes } from "./format";
import type { Operation } from "./useOperation";
import { OperationProgress } from "./OperationProgress";

export interface ProductActions {
  onModify: () => void;
  onLaunch: () => void;
  onUpdate: () => void;
  onUninstall: () => void;
  onRepair: () => void;
  onPause: (paused: boolean) => void;
  onCancel: () => void;
  onDismiss: () => void;
}

/** A product as Visual Studio Installer shows one: installed (Installed tab) or to install (Available). */
export function ProductCard({ product, op, busy, actions }: { product: Product; op: Operation | null; busy: boolean; actions: ProductActions }) {
  const t = useT();
  const more = useContextMenu<void>();
  const installed = product.installed !== null;
  const running = op !== null && op.finished === null;
  const version = installed ? product.installed : product.latest;

  return (
    <article className="yi-card">
      <div className="yi-card-icon">{product.icon ? <img src={product.icon} alt="" width={40} height={40} /> : <Icon name="package" size={36} />}</div>
      <div className="yi-card-main">
        <h2 className="yi-card-name">{product.name}</h2>
        {version && <div className="yi-card-version">{version}</div>}
        <p className="yi-card-description">{t(product.description)}</p>
        {product.notesUrl && (
          <button type="button" className="yi-link" onClick={() => void openLink(product.notesUrl as string)}>
            {t("Release notes")}
          </button>
        )}
        {installed && product.updateAvailable && !running && (
          <div className="yi-card-update">
            <span>{t("Update available")}</span>
            <span className="yi-card-update-version">{product.latest}</span>
          </div>
        )}
        {!installed && product.latest === null && <p className="yi-op-error">{t("Its versions couldn't be read — see the notifications.")}</p>}
        {op && <OperationProgress op={op} onPause={actions.onPause} onDismiss={actions.onDismiss} />}
      </div>
      <div className="yi-card-actions">
        {!installed && (
          <Button variant="primary" disabled={busy || product.latest === null} onClick={actions.onModify}>
            {t("Install")}
          </Button>
        )}
        {installed && product.updateAvailable && (
          <Button variant="primary" disabled={busy} onClick={actions.onUpdate}>
            {t("Update")}
          </Button>
        )}
        {installed && (
          <>
            <Button disabled={busy} onClick={actions.onModify}>
              {t("Modify")}
            </Button>
            <Button disabled={running} onClick={actions.onLaunch}>
              {t("Launch")}
            </Button>
            <div className="yi-more-anchor">
              <Button onClick={() => more.openAtAnchor()}>
                {t("More")} <Icon name="chevronRight" size={11} className="yi-more-chevron" />
              </Button>
              <ContextMenu
                target={more.target ? { mode: "anchor" } : null}
                items={[
                  { label: t("Repair"), onSelect: actions.onRepair, disabled: busy },
                  { label: t("Uninstall"), onSelect: actions.onUninstall, disabled: busy },
                  { label: t("Cancel"), onSelect: actions.onCancel, disabled: !running },
                ]}
                onClose={more.close}
              />
            </div>
          </>
        )}
        {!installed && product.size !== null && <span className="yi-card-size">{formatBytes(product.size)}</span>}
      </div>
    </article>
  );
}
