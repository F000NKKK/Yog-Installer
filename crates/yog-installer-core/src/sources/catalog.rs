//! The catalog (the channel): products, workloads, components and their
//! groups, and where each component is published. See `catalog/catalog.toml`.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

/// The catalog this Installer was built with.
pub const BUILT_IN: &str = include_str!("../../../../catalog/catalog.toml");

/// Where a component's versions are published, and so how it's installed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Source {
    /// A GitHub repo's releases; per platform tag, the asset whose name ends so.
    Github {
        repo: String,
        assets: BTreeMap<String, String>,
        /// A package running on the system's own libraries, taken instead
        /// where the system has them (see [`SystemPackage`]).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        system: Option<SystemPackage>,
    },
    /// A GitHub repo's release tags, installed from the tag's source.
    Git { repo: String },
    /// A crates.io crate, `cargo install`ed.
    Crate { name: String },
    /// A Rust toolchain, through rustup.
    Rustup { toolchain: String },
    /// Rust targets of a toolchain, through rustup.
    RustTargets {
        toolchain: String,
        targets: Vec<String>,
    },
    /// A Temurin JDK of a Java major version, from Adoptium.
    Adoptium { major: u32 },
    /// The newest stable zig, from ziglang.org.
    Zig,
}

/// A package of a product built against the system's libraries (a Linux
/// `.deb`'s program): the system's WebKit and GStreamer, not the older
/// ones an AppImage carries, which break against the system's plugins
/// (sound dying on Bluetooth outputs, no GPU compositing).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemPackage {
    /// Per platform tag, the asset whose name ends so.
    pub assets: BTreeMap<String, String>,
    /// The libraries (sonames) the system must have for it.
    pub needs: Vec<String>,
}

impl Source {
    /// How the name of the asset this platform installs ends, if any: the
    /// system package's where it fits.
    pub fn asset_suffix(&self) -> Option<&str> {
        let Source::Github { assets, system, .. } = self else {
            return None;
        };
        let host = yog_ids::platform::host_tag();
        system
            .as_ref()
            .filter(|s| crate::machine::platform::has_libraries(&s.needs))
            .and_then(|s| s.assets.get(&host))
            .or_else(|| assets.get(&host))
            .map(String::as_str)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub description: String,
    pub workloads: Vec<String>,
    /// The program that starts it (on macOS, the `.app` inside its package).
    pub program: String,
    /// Its icon (a PNG's URL), for its card and its desktop entry.
    #[serde(default)]
    pub icon: Option<String>,
    pub source: Source,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workload {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub icon: Option<String>,
    /// Always installed with the workload.
    pub required: Vec<String>,
    /// Checked by default, can be unchecked.
    #[serde(default)]
    pub recommended: Vec<String>,
    /// Unchecked by default.
    #[serde(default)]
    pub optional: Vec<String>,
}

impl Workload {
    /// Its components: required, then recommended, then optional.
    pub fn components(&self) -> impl Iterator<Item = &String> {
        self.required
            .iter()
            .chain(&self.recommended)
            .chain(&self.optional)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    pub name: String,
    pub description: String,
    pub group: String,
    /// Components it can't work without.
    #[serde(default)]
    pub requires: Vec<String>,
    pub source: Source,
}

/// Yog Installer itself, as a product of its own channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Installer {
    pub name: String,
    pub program: String,
    pub source: Source,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Catalog {
    /// Which is newer, of two catalogs.
    #[serde(default)]
    pub revision: u32,
    pub installer: Option<Installer>,
    #[serde(default, rename = "group")]
    pub groups: Vec<Group>,
    #[serde(default, rename = "product")]
    pub products: Vec<Product>,
    #[serde(default, rename = "workload")]
    pub workloads: Vec<Workload>,
    #[serde(default, rename = "component")]
    pub components: Vec<Component>,
}

impl Catalog {
    /// Reads and checks a catalog.
    pub fn parse(text: &str) -> Result<Self, String> {
        let catalog: Self = toml::from_str(text).map_err(|e| format!("catalog: {e}"))?;
        catalog.check()?;
        Ok(catalog)
    }

    /// The catalog built in (its test checks it parses; empty if it didn't).
    pub fn built_in() -> Self {
        Self::parse(BUILT_IN).unwrap_or_default()
    }

    pub fn product(&self, id: &str) -> Option<&Product> {
        self.products.iter().find(|p| p.id == id)
    }

    pub fn workload(&self, id: &str) -> Option<&Workload> {
        self.workloads.iter().find(|w| w.id == id)
    }

    pub fn component(&self, id: &str) -> Option<&Component> {
        self.components.iter().find(|c| c.id == id)
    }

    /// Ids are unique and every reference names something there.
    fn check(&self) -> Result<(), String> {
        let mut ids = HashSet::new();
        let all = self
            .groups
            .iter()
            .map(|g| &g.id)
            .chain(self.products.iter().map(|p| &p.id));
        let all = all
            .chain(self.workloads.iter().map(|w| &w.id))
            .chain(self.components.iter().map(|c| &c.id));
        for id in all {
            if !ids.insert(id) {
                return Err(format!("catalog: `{id}` is there twice"));
            }
        }
        let missing = |kind: &str, owner: &str, id: &str| {
            format!("catalog: {owner} names {kind} `{id}`, which isn't there")
        };
        for p in &self.products {
            if let Some(w) = p.workloads.iter().find(|w| self.workload(w).is_none()) {
                return Err(missing("workload", &p.id, w));
            }
        }
        for w in &self.workloads {
            if let Some(c) = w.components().find(|c| self.component(c).is_none()) {
                return Err(missing("component", &w.id, c));
            }
        }
        for c in &self.components {
            if !self.groups.iter().any(|g| g.id == c.group) {
                return Err(missing("group", &c.id, &c.group));
            }
            if let Some(r) = c.requires.iter().find(|r| self.component(r).is_none()) {
                return Err(missing("component", &c.id, r));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_catalog_is_whole() {
        let c = Catalog::parse(BUILT_IN).unwrap();
        assert!(c.product("yog.idle").is_some());
        assert_eq!(
            c.component("yog.jdk.21").unwrap().source,
            Source::Adoptium { major: 21 }
        );
        // The ids yog-home looks for are the catalog's.
        assert!(c.component(yog_home::components::id::LOADER_SDK).is_some());
        assert!(c.component(&yog_home::components::id::jdk(25)).is_some());
    }

    #[test]
    fn a_dangling_reference_is_named() {
        let text = "[[group]]\nid = \"g\"\nname = \"G\"\n\n[[component]]\nid = \"a\"\nname = \"A\"\ndescription = \"\"\ngroup = \"g\"\nrequires = [\"b\"]\nsource = { kind = \"zig\" }\n";
        assert_eq!(
            Catalog::parse(text).unwrap_err(),
            "catalog: a names component `b`, which isn't there"
        );
    }
}
