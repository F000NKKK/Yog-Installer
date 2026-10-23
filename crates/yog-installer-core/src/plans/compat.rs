//! Versions and what they need of each other.
//!
//! A release may carry `yog-release.json` — `{"requires": {"yog.cli":
//! ">=0.47.0"}}` — what it needs of the other components. A selection is
//! checked against it before anything is installed.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub use yog_home::version::{compare, meets};

/// The asset a release says its needs in.
pub const RELEASE_MANIFEST: &str = "yog-release.json";

/// A release's needs: component id → version requirement (semver: `>=0.47.0, <0.48`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Needs {
    #[serde(default)]
    pub requires: BTreeMap<String, String>,
}

/// A need that isn't met.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Conflict {
    /// Who needs it, at which version.
    pub component: String,
    pub version: String,
    /// What it needs, and what there is (none: not installed nor chosen).
    pub needs: String,
    pub requirement: String,
    pub found: Option<String>,
}

/// Every need of `chosen` (id → version, with its needs) that `versions`
/// (id → the version there will be) doesn't meet.
pub fn check(
    chosen: &[(String, String, Needs)],
    versions: &BTreeMap<String, String>,
) -> Vec<Conflict> {
    let mut out = Vec::new();
    for (component, version, needs) in chosen {
        for (other, requirement) in &needs.requires {
            let found = versions.get(other).cloned();
            if !found.as_deref().is_some_and(|v| meets(v, requirement)) {
                out.push(Conflict {
                    component: component.clone(),
                    version: version.clone(),
                    needs: other.clone(),
                    requirement: requirement.clone(),
                    found,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unmet_needs_are_conflicts() {
        let needs = Needs {
            requires: [("yog.cli".to_string(), ">=0.47.0".to_string())].into(),
        };
        let chosen = vec![("yog.idle".to_string(), "2.0.0".to_string(), needs)];
        let old: BTreeMap<_, _> = [("yog.cli".to_string(), "0.46.2".to_string())].into();
        let c = check(&chosen, &old);
        assert_eq!((c.len(), c[0].found.as_deref()), (1, Some("0.46.2")));
        let new: BTreeMap<_, _> = [("yog.cli".to_string(), "0.47.1".to_string())].into();
        assert!(check(&chosen, &new).is_empty());
        assert_eq!(check(&chosen, &BTreeMap::new())[0].found, None);
    }
}
