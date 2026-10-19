import { openLink, useT } from "substrate-platform-ui";
import type { News } from "./api";

/** Where "Need help?" leads. */
const HELP_LINKS = [
  { label: "Yog documentation", url: "https://github.com/F000NKKK/Yog-Mod-Loader#readme" },
  { label: "Report a problem", url: "https://github.com/F000NKKK/Yog-Installer/issues/new" },
  { label: "Yog on GitHub", url: "https://github.com/F000NKKK" },
] as const;

/** The right column, as Visual Studio Installer's: the developer news, then where to get help. */
export function NewsPanel({ news, locale }: { news: News[]; locale: string }) {
  const t = useT();
  const date = (iso: string | null) => (iso ? new Date(iso).toLocaleDateString(locale, { day: "2-digit", month: "2-digit", year: "numeric" }) : "");
  return (
    <aside className="yi-news">
      <h2>{t("Developer News")}</h2>
      {news.length === 0 && <p className="yi-muted">{t("No news yet.")}</p>}
      {news.map((n) => (
        <article key={`${n.of}-${n.version}`} className="yi-news-item">
          <button type="button" className="yi-link yi-news-title" disabled={!n.url} onClick={() => n.url && void openLink(n.url)}>
            {n.title ?? `${n.of} ${n.version}`}
          </button>
          <div className="yi-muted yi-news-meta">
            <span className="yi-news-of">
              {n.of} · {n.version}
            </span>
            {n.published && <time dateTime={n.published}>{date(n.published)}</time>}
          </div>
        </article>
      ))}
      <h2 className="yi-help-heading">{t("Need help?")}</h2>
      {HELP_LINKS.map((l) => (
        <button key={l.url} type="button" className="yi-link yi-help-link" onClick={() => void openLink(l.url)}>
          {t(l.label)}
        </button>
      ))}
    </aside>
  );
}
