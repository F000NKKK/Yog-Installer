import { useT } from "substrate-platform-ui";

/** The languages Yog's products speak — built into them, nothing to download. */
const LANGUAGES = [
  { id: "en", name: "English" },
  { id: "ru", name: "Русский" },
] as const;

export function LanguagePacksTab() {
  const t = useT();
  return (
    <div className="yi-languages">
      <p className="yi-muted">{t("Every language comes with the product: choose the one it speaks in its Options.")}</p>
      {LANGUAGES.map((l) => (
        <label key={l.id} className="yi-component">
          <input type="checkbox" checked disabled />
          <span className="yi-component-name">{l.name}</span>
        </label>
      ))}
    </div>
  );
}
