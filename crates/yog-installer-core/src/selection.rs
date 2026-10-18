//! What's chosen in the Modify window — workloads, their recommended and
//! optional parts, individual components — and what installing it changes.

use crate::catalog::Catalog;
use crate::detect::Found;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The Modify window's checkboxes.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Selection {
    pub workloads: BTreeSet<String>,
    /// A workload's recommended components left unchecked.
    pub unchecked: BTreeSet<String>,
    /// A workload's optional components checked, and individual components.
    pub checked: BTreeSet<String>,
}

impl Selection {
    /// What's installed, as checkboxes: a workload is checked when all it
    /// requires is there; every other installed component is checked.
    pub fn of_installed(catalog: &Catalog, product: &str, installed: &[Found]) -> Self {
        let has = |id: &String| installed.iter().any(|f| &f.id == id);
        let mut s = Self::default();
        if let Some(p) = catalog.product(product) {
            for w in p.workloads.iter().filter_map(|w| catalog.workload(w)) {
                if w.required.iter().all(has) {
                    s.workloads.insert(w.id.clone());
                    s.unchecked
                        .extend(w.recommended.iter().filter(|c| !has(c)).cloned());
                }
            }
        }
        let covered = s.components(catalog);
        s.checked.extend(
            installed
                .iter()
                .map(|f| f.id.clone())
                .filter(|id| !covered.contains(id)),
        );
        s
    }

    /// Every component it makes: the workloads' required and still-checked
    /// recommended ones, the checked ones, and all they require.
    pub fn components(&self, catalog: &Catalog) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for w in self.workloads.iter().filter_map(|w| catalog.workload(w)) {
            out.extend(w.required.iter().cloned());
            out.extend(
                w.recommended
                    .iter()
                    .filter(|c| !self.unchecked.contains(*c))
                    .cloned(),
            );
        }
        out.extend(self.checked.iter().cloned());
        let mut todo: Vec<String> = out.iter().cloned().collect();
        while let Some(id) = todo.pop() {
            for r in catalog
                .component(&id)
                .map(|c| c.requires.clone())
                .unwrap_or_default()
            {
                if out.insert(r.clone()) {
                    todo.push(r);
                }
            }
        }
        out
    }
}

/// One thing a plan does.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Change {
    Install {
        id: String,
        version: String,
    },
    Update {
        id: String,
        from: String,
        to: String,
    },
    Remove {
        id: String,
        version: String,
    },
}

impl Change {
    pub fn id(&self) -> &str {
        match self {
            Change::Install { id, .. } | Change::Update { id, .. } | Change::Remove { id, .. } => {
                id
            }
        }
    }
}

/// What makes `wanted` installed at `latest` (id → its newest version):
/// installs and updates, what a component requires first; then removals of
/// what the Installer put there and isn't wanted (what was there before it stays).
pub fn changes(
    catalog: &Catalog,
    wanted: &BTreeSet<String>,
    installed: &[Found],
    latest: &BTreeMap<String, String>,
) -> Vec<Change> {
    let mut out = Vec::new();
    for id in in_order(catalog, wanted) {
        let Some(to) = latest.get(&id) else { continue };
        match installed.iter().find(|f| f.id == id) {
            None => out.push(Change::Install {
                id,
                version: to.clone(),
            }),
            Some(f) if f.managed && crate::compat::compare(to, &f.version).is_gt() => {
                out.push(Change::Update {
                    id,
                    from: f.version.clone(),
                    to: to.clone(),
                })
            }
            Some(_) => {}
        }
    }
    for f in installed
        .iter()
        .filter(|f| f.managed && !wanted.contains(&f.id))
    {
        out.push(Change::Remove {
            id: f.id.clone(),
            version: f.version.clone(),
        });
    }
    out
}

/// `ids` with every component after the ones it requires.
fn in_order(catalog: &Catalog, ids: &BTreeSet<String>) -> Vec<String> {
    fn visit(catalog: &Catalog, id: &str, ids: &BTreeSet<String>, out: &mut Vec<String>) {
        if out.iter().any(|o| o == id) || !ids.contains(id) {
            return;
        }
        for r in catalog
            .component(id)
            .map(|c| c.requires.clone())
            .unwrap_or_default()
        {
            visit(catalog, &r, ids, out);
        }
        out.push(id.to_string());
    }
    let mut out = Vec::new();
    for id in ids {
        visit(catalog, id, ids, &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(id: &str, version: &str, managed: bool) -> Found {
        Found {
            id: id.into(),
            version: version.into(),
            path: None,
            managed,
        }
    }

    #[test]
    fn a_workload_brings_its_parts_and_what_they_require() {
        let catalog = Catalog::built_in();
        let mut s = Selection::default();
        s.workloads.insert("yog.workload.mods".into());
        s.unchecked.insert("yog.sdk.loader".into());
        let all = s.components(&catalog);
        assert!(
            all.contains("yog.cli") && all.contains("rust.targets") && all.contains("yog.jdk.21")
        );
        assert!(!all.contains("yog.sdk.loader") && !all.contains("yog.jdk.17"));
    }

    #[test]
    fn changes_install_requirements_first_and_keep_the_systems() {
        let catalog = Catalog::built_in();
        let wanted: BTreeSet<String> = ["yog.cli".into(), "rust.toolchain".into()].into();
        let latest: BTreeMap<String, String> = [
            ("yog.cli".into(), "0.47.1".into()),
            ("rust.toolchain".into(), "1.99".into()),
        ]
        .into();
        let none = changes(&catalog, &wanted, &[], &latest);
        assert_eq!(
            none.iter().map(Change::id).collect::<Vec<_>>(),
            ["rust.toolchain", "yog.cli"]
        );

        let installed = [
            found("rust.toolchain", "1.99", false),
            found("yog.cli", "0.47.0", true),
            found("yog.jdk.17", "17.0.13", true),
            found("zig", "0.13.0", false),
        ];
        let c = changes(&catalog, &wanted, &installed, &latest);
        assert_eq!(
            c,
            [
                Change::Update {
                    id: "yog.cli".into(),
                    from: "0.47.0".into(),
                    to: "0.47.1".into()
                },
                Change::Remove {
                    id: "yog.jdk.17".into(),
                    version: "17.0.13".into()
                },
            ]
        );
    }

    #[test]
    fn the_installed_read_back_as_checkboxes() {
        let catalog = Catalog::built_in();
        let installed = [
            found("yog.cli", "0.47.1", false),
            found("rust.toolchain", "1.99", false),
            found("yog.jdk.17", "17", true),
        ];
        let s = Selection::of_installed(&catalog, "yog.idle", &installed);
        assert!(s.workloads.contains("yog.workload.mods"));
        assert!(s.unchecked.contains("rust.targets"));
        assert!(s.checked.contains("yog.jdk.17"));
    }
}
