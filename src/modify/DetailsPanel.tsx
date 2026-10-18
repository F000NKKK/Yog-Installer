import { Icon, useT } from "substrate-platform-ui";
import type { Component, Why, Workload } from "../api";
import { clearable } from "./useModify";

/** Installation details: each chosen workload's included and optional parts, then the individual components. */
export function DetailsPanel({ workloads, components, reasons, onSet }: { workloads: Workload[]; components: Component[]; reasons: Record<string, Why>; onSet: (id: string, on: boolean, why: Why | undefined) => void }) {
  const t = useT();
  const name = (id: string) => t(components.find((c) => c.id === id)?.name ?? id);
  const inWorkloads = new Set(workloads.flatMap((w) => [...w.required, ...w.recommended, ...w.optional]));
  const individual = Object.entries(reasons).filter(([id]) => !inWorkloads.has(id));

  const row = (id: string) => {
    const why = reasons[id];
    const on = why !== undefined;
    return (
      <label key={id} className="yi-detail-row">
        <input type="checkbox" checked={on} disabled={on && !clearable(why)} onChange={(e) => onSet(id, e.target.checked, why)} />
        {name(id)}
      </label>
    );
  };

  return (
    <aside className="yi-details">
      <h3>{t("Installation details")}</h3>
      {workloads.map((w) => (
        <details key={w.id} open className="yi-detail">
          <summary>
            <Icon name="chevronRight" size={11} className="yi-detail-chevron" /> {t(w.name)}
          </summary>
          <div className="yi-detail-heading">{t("Included")}</div>
          {w.required.map((id) => (
            <div key={id} className="yi-detail-row yi-detail-row--included">
              <Icon name="check" size={12} /> {name(id)}
            </div>
          ))}
          {w.recommended.length + w.optional.length > 0 && <div className="yi-detail-heading">{t("Optional")}</div>}
          {[...w.recommended, ...w.optional].map(row)}
        </details>
      ))}
      {individual.length > 0 && (
        <details open className="yi-detail">
          <summary>
            <Icon name="chevronRight" size={11} className="yi-detail-chevron" /> {t("Individual components")}
          </summary>
          {individual.map(([id]) => row(id))}
        </details>
      )}
      {workloads.length === 0 && individual.length === 0 && <p className="yi-muted">{t("Nothing selected.")}</p>}
    </aside>
  );
}
