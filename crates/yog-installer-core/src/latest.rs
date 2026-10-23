//! What's published now: the newest release of everything in the catalog
//! (the Installer too), asked side by side, and the developer news — the
//! latest releases of Yog's own repos — from the same answers.

use crate::catalog::{Catalog, Source};
use crate::releases::{self, Release};
use serde::Serialize;
use std::collections::BTreeMap;
use yog_home::components::id::INSTALLER;

/// Releases of each repo shown as news, and news items in all.
const NEWS_PER_SOURCE: usize = 3;
const NEWS_MAX: usize = 6;

/// A release in the developer news.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct News {
    /// What it's a release of (`Yog IDLE`).
    pub of: String,
    pub version: String,
    pub title: Option<String>,
    pub published: Option<String>,
    pub url: Option<String>,
}

/// What asking gave.
#[derive(Debug, Default)]
pub struct Latest {
    /// Id → its newest release.
    pub newest: BTreeMap<String, Release>,
    /// Newest first.
    pub news: Vec<News>,
    /// What couldn't be asked.
    pub problems: Vec<String>,
}

/// Asks every source of `catalog`.
pub fn fetch(catalog: &Catalog) -> Latest {
    let products = catalog
        .products
        .iter()
        .map(|p| (p.id.clone(), p.name.clone(), p.source.clone()));
    let components = catalog
        .components
        .iter()
        .map(|c| (c.id.clone(), c.name.clone(), c.source.clone()));
    let installer = catalog
        .installer
        .iter()
        .map(|i| (INSTALLER.to_string(), i.name.clone(), i.source.clone()));
    let jobs: Vec<_> = products
        .chain(components)
        .chain(installer)
        .map(|(id, name, source)| {
            std::thread::spawn(move || (id, name, releases::of(&source), source))
        })
        .collect();
    let mut out = Latest::default();
    for job in jobs {
        match job.join() {
            Ok((id, name, Ok(list), source)) => {
                if matches!(source, Source::Github { .. } | Source::Git { .. }) {
                    out.news
                        .extend(list.iter().take(NEWS_PER_SOURCE).map(|r| News {
                            of: name.clone(),
                            version: r.version.clone(),
                            title: r.title.clone(),
                            published: r.published.clone(),
                            url: r.notes_url.clone(),
                        }));
                }
                // A release without this platform's file (not built yet, or
                // failed) isn't one to install.
                if let Some(newest) = list.into_iter().find(|r| releases::carries(&source, r)) {
                    out.newest.insert(id, newest);
                }
            }
            Ok((id, _, Err(e), _)) => out.problems.push(format!("{id}: {e}")),
            Err(_) => out.problems.push("a version check stopped".into()),
        }
    }
    // RFC 3339 dates order as text.
    out.news.sort_by(|a, b| b.published.cmp(&a.published));
    out.news.truncate(NEWS_MAX);
    out
}
