import { Icon, useT } from "substrate-platform-ui";
import type { Workload } from "../api";

/** Workloads: a card each, checked to install it — as Visual Studio Installer lays them out. */
export function WorkloadsTab({ workloads, chosen, onToggle }: { workloads: Workload[]; chosen: string[]; onToggle: (id: string) => void }) {
  const t = useT();
  return (
    <div className="yi-workloads">
      {workloads.map((w) => {
        const on = chosen.includes(w.id);
        return (
          <label key={w.id} className={`yi-workload${on ? " is-checked" : ""}`}>
            <div className="yi-workload-icon">
              <Icon name={w.icon ?? "package"} size={32} />
            </div>
            <div className="yi-workload-text">
              <span className="yi-workload-name">{t(w.name)}</span>
              <span className="yi-workload-description">{t(w.description)}</span>
            </div>
            <input type="checkbox" className="yi-workload-check" checked={on} onChange={() => onToggle(w.id)} />
          </label>
        );
      })}
    </div>
  );
}
