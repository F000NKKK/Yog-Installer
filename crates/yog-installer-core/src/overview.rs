//! What the window shows: the products and their state, and for the Modify
//! window every workload and component with what's installed and newest.

use crate::catalog::Catalog;
use crate::compat::{self, Conflict, Needs, RELEASE_MANIFEST};
use crate::detect::Found;
use crate::engine;
use crate::install;
use crate::releases::Release;
use crate::selection::Change;
use serde::Serialize;
use std::collections::BTreeMap;

/// A product card.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductView {
    pub id: String,
    pub name: String,
    pub description: String,
    pub installed: Option<String>,
    pub latest: Option<String>,
    pub update_available: bool,
    pub notes_url: Option<String>,
    /// What starts it, when installed.
    pub program: Option<String>,
    /// Its download for this platform (bytes), when known.
    pub size: Option<u64>,
    /// Its icon's URL.
    pub icon: Option<String>,
}

pub fn products(
    catalog: &Catalog,
    installed: &[Found],
    latest: &BTreeMap<String, Release>,
) -> Vec<ProductView> {
    catalog
        .products
        .iter()
        .map(|p| {
            let have = installed.iter().find(|f| f.id == p.id);
            let newest = latest.get(&p.id);
            let item = install::Item::Product(p);
            ProductView {
                id: p.id.clone(),
                name: p.name.clone(),
                description: p.description.clone(),
                installed: have.map(|f| f.version.clone()),
                latest: newest.map(|r| r.version.clone()),
                update_available: matches!((have, newest), (Some(f), Some(r)) if compat::compare(&r.version, &f.version).is_gt()),
                notes_url: newest.and_then(|r| r.notes_url.clone()),
                program: have.and_then(|f| f.path.as_ref()).map(|p| p.display().to_string()),
                size: newest.and_then(|r| install::asset(&item, r)).map(|a| a.size),
                icon: p.icon.clone(),
            }
        })
        .collect()
}

/// A component's line in the Modify window.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentView {
    pub id: String,
    pub name: String,
    pub description: String,
    pub group: String,
    pub installed: Option<String>,
    /// Installed by the Installer (it can update and remove it).
    pub managed: bool,
    pub latest: Option<String>,
    pub size: Option<u64>,
}

pub fn components(
    catalog: &Catalog,
    installed: &[Found],
    latest: &BTreeMap<String, Release>,
) -> Vec<ComponentView> {
    catalog
        .components
        .iter()
        .map(|c| {
            let have = installed.iter().find(|f| f.id == c.id);
            let newest = latest.get(&c.id);
            let item = install::Item::Component {
                id: &c.id,
                source: &c.source,
            };
            ComponentView {
                id: c.id.clone(),
                name: c.name.clone(),
                description: c.description.clone(),
                group: c.group.clone(),
                installed: have.map(|f| f.version.clone()),
                managed: have.is_some_and(|f| f.managed),
                latest: newest.map(|r| r.version.clone()),
                size: newest
                    .and_then(|r| install::asset(&item, r))
                    .map(|a| a.size),
            }
        })
        .collect()
}

/// What the plan's releases need of each other that the outcome won't have:
/// each release's `yog-release.json`, against the versions there'll be.
pub fn conflicts(catalog: &Catalog, plan: &engine::Plan, installed: &[Found]) -> Vec<Conflict> {
    let mut versions: BTreeMap<String, String> = installed
        .iter()
        .map(|f| (f.id.clone(), f.version.clone()))
        .collect();
    for change in &plan.changes {
        match change {
            Change::Install { id, version }
            | Change::Update {
                id, to: version, ..
            } => {
                versions.insert(id.clone(), version.clone());
            }
            Change::Remove { id, .. } => {
                versions.remove(id);
            }
        }
    }
    let chosen: Vec<(String, String, Needs)> = plan
        .releases
        .iter()
        .filter(|(id, _)| engine::item(catalog, id).is_some() && versions.contains_key(*id))
        .filter_map(|(id, r)| {
            let manifest = r.asset(RELEASE_MANIFEST)?;
            let needs: Needs = crate::http::get_json(&manifest.url).ok()?;
            Some((id.clone(), r.version.clone(), needs))
        })
        .collect();
    compat::check(&chosen, &versions)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Asks every source of the built-in catalog (network: `cargo test -- --ignored`).
    #[test]
    #[ignore]
    fn every_source_answers() {
        let catalog = Catalog::built_in();
        let crate::latest::Latest {
            newest: latest,
            problems,
            news,
        } = crate::latest::fetch(&catalog);
        assert!(problems.is_empty(), "{problems:?}");
        assert!(!news.is_empty());
        for c in &catalog.components {
            assert!(latest.contains_key(&c.id), "{} has no release", c.id);
        }
        let idle = &products(&catalog, &[], &latest)[0];
        assert!(idle.latest.is_some() && idle.size.is_some(), "{idle:?}");
    }
}
