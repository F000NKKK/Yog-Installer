//! Installing and removing one component (or product), by its source kind:
//! downloaded files are unpacked into Yog's home (`files`), tools go through
//! rustup and cargo (`tools`).

pub mod archive;
mod files;
pub mod run;
mod tools;

use crate::catalog::{Product, Source};
use crate::platform;
use crate::progress::{Cancel, Event, Sink};
use crate::releases::{Asset, Release, SOURCE_ARCHIVE};
use std::path::{Path, PathBuf};
use yog_home::Home;

/// Where zig goes, under the home's tools.
const ZIG: &str = "zig";

/// What installing has to hand.
pub struct Context<'a> {
    pub home: &'a Home,
    pub cancel: &'a Cancel,
    pub sink: &'a dyn Sink,
    /// cargo's and rustup's folder.
    pub cargo_bin: PathBuf,
}

impl Context<'_> {
    /// A line a tool printed, for `id`.
    fn output(&self, id: &str) -> impl Fn(String) + '_ {
        let id = id.to_string();
        move |line| {
            self.sink.event(Event::Output {
                component: id.clone(),
                line,
            })
        }
    }
}

/// What a component is installed from: its catalog source, or the product it is.
pub enum Item<'a> {
    Component { id: &'a str, source: &'a Source },
    Product(&'a Product),
}

impl Item<'_> {
    pub fn id(&self) -> &str {
        match self {
            Item::Component { id, .. } => id,
            Item::Product(p) => &p.id,
        }
    }

    pub fn source(&self) -> &Source {
        match self {
            Item::Component { source, .. } => source,
            Item::Product(p) => &p.source,
        }
    }
}

/// The file `item` at `release` is installed from on this platform; none
/// for what's installed by a tool (rustup, cargo).
pub fn asset<'r>(item: &Item, release: &'r Release) -> Option<&'r Asset> {
    match item.source() {
        Source::Github { assets, .. } => assets
            .get(&platform::tag())
            .and_then(|suffix| release.asset_ending(suffix)),
        Source::Git { .. } => release.asset(SOURCE_ARCHIVE),
        Source::Adoptium { .. } | Source::Zig => release.assets.first(),
        Source::Crate { .. } | Source::Rustup { .. } | Source::RustTargets { .. } => None,
    }
}

/// Installs `item` at `release` (`download`: its asset, already fetched);
/// where it went, for `components.json`.
pub fn install(
    item: &Item,
    release: &Release,
    download: Option<&Path>,
    ctx: &Context,
) -> Result<PathBuf, String> {
    let need = || download.ok_or_else(|| format!("{}: nothing downloaded to install", item.id()));
    let version = &release.version;
    match (item, item.source()) {
        (Item::Product(p), _) => files::product(
            p,
            version,
            need()?,
            asset(item, release).map(|a| a.name.as_str()),
            ctx,
        ),
        (_, Source::Git { .. }) => files::unpacked(
            need()?,
            SOURCE_ARCHIVE,
            &ctx.home.loader_sdk_dir(version),
            ctx,
        ),
        (_, Source::Adoptium { major }) => files::jdk(
            need()?,
            asset(item, release),
            &ctx.home.jdk_dir(*major, version),
            ctx,
        ),
        (_, Source::Zig) => files::unpacked(
            need()?,
            &asset(item, release).map_or(String::new(), |a| a.name.clone()),
            &ctx.home.tool_dir(ZIG, version),
            ctx,
        ),
        (_, Source::Crate { name }) => tools::cargo_install(item.id(), name, version, ctx),
        (_, Source::Rustup { toolchain }) => tools::toolchain(item.id(), toolchain, ctx),
        (_, Source::RustTargets { toolchain, targets }) => {
            tools::targets(item.id(), toolchain, targets, ctx)
        }
        (Item::Component { .. }, Source::Github { .. }) => Err(format!(
            "{}: a GitHub release is installed as a product",
            item.id()
        )),
    }
}

/// Removes `item` at `version`, installed at `path`.
pub fn remove(item: &Item, version: &str, path: &Path, ctx: &Context) -> Result<(), String> {
    match (item, item.source()) {
        (Item::Product(p), _) => {
            files::remove_product(p, &ctx.home.product_dir(&p.id, version), path, ctx)
        }
        (_, Source::Crate { name }) => tools::cargo_uninstall(item.id(), name, ctx),
        (_, Source::Rustup { toolchain }) => tools::toolchain_uninstall(item.id(), toolchain, ctx),
        (_, Source::RustTargets { toolchain, targets }) => {
            tools::targets_remove(item.id(), toolchain, targets, ctx)
        }
        (_, Source::Adoptium { major }) => files::remove_dir(&ctx.home.jdk_dir(*major, version)),
        _ => files::remove_dir(path),
    }
}
