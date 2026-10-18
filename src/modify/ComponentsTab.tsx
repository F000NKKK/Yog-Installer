import { useState } from "react";
import { SearchField, useT } from "substrate-platform-ui";
import type { Component, Group, Why } from "../api";
import { formatBytes } from "../format";
import { clearable } from "./useModify";

/** Individual components: searchable, by group, a checkbox each. */
export function ComponentsTab({ components, groups, reasons, onSet }: { components: Component[]; groups: Group[]; reasons: Record<string, Why>; onSet: (id: string, on: boolean, why: Why | undefined) => void }) {
  const t = useT();
  const [query, setQuery] = useState("");
  const q = query.trim().toLowerCase();
  const shown = components.filter((c) => !q || `${t(c.name)} ${c.id} ${t(c.description)}`.toLowerCase().includes(q));

  return (
    <div className="yi-components">
      <SearchField value={query} onValueChange={setQuery} clearable placeholder={t("Search components (Ctrl+Q)")} />
      {groups.map((g) => {
        const members = shown.filter((c) => c.group === g.id);
        if (members.length === 0) return null;
        return (
          <section key={g.id} className="yi-component-group">
            <h3>{t(g.name)}</h3>
            {members.map((c) => {
              const why = reasons[c.id];
              const on = why !== undefined;
              return (
                <label key={c.id} className="yi-component" title={t(c.description)}>
                  <input type="checkbox" checked={on} disabled={on && !clearable(why)} onChange={(e) => onSet(c.id, e.target.checked, why)} />
                  <span className="yi-component-name">{t(c.name)}</span>
                  <span className="yi-component-meta">
                    {c.installed ? t("installed {version}", { version: c.installed }) : (c.latest ?? "")}
                    {c.size ? ` · ${formatBytes(c.size)}` : ""}
                  </span>
                </label>
              );
            })}
          </section>
        );
      })}
    </div>
  );
}
