//! Installing and removing one component (or product), by its source kind:
//! downloaded files are unpacked into Yog's home (`files`), tools go through
//! rustup and cargo (`tools`).

pub mod archive;
mod files;
pub mod launcher;
pub mod run;
mod tools;

use crate::catalog::{Product, Source};
use crate::progress::{Cancel, Event, Sink};
use crate::releases::{Asset, Release, SOURCE_ARCHIVE};
use archive::Kind;
use std::path::{Path, PathBuf};

pub(crate) use files::make_executable;
use yog_home::Home;

/// Where zig goes, under the home's tools.
const ZIG: &str = "zig";
/// What an unpacking folder's name ends with until it's moved into place.
const STAGING_SUFFIX: &str = ".part";

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
            .get(&yog_ids::platform::host_tag())
            .and_then(|suffix| release.asset_ending(suffix)),
        Source::Git { .. } => release.asset(SOURCE_ARCHIVE),
        Source::Adoptium { .. } | Source::Zig => release.assets.first(),
        Source::Crate { .. } | Source::Rustup { .. } | Source::RustTargets { .. } => None,
    }
}

/// An archive download's way in: unpacked into `staging` beside its place
/// (while it downloads, or from the file), then moved there ([`finish`]).
pub struct Unpacking {
    pub kind: Kind,
    pub staging: PathBuf,
    /// Without the one folder the archive's entries are in.
    pub strip: bool,
}

/// Where `item` at `release` goes in Yog's home, when its download is unpacked there.
fn place(item: &Item, version: &str, ctx: &Context) -> Option<PathBuf> {
    match (item, item.source()) {
        (Item::Product(p), _) => Some(ctx.home.product_dir(&p.id, version)),
        (_, Source::Git { .. }) => Some(ctx.home.loader_sdk_dir(version)),
        (_, Source::Adoptium { major }) => Some(ctx.home.jdk_dir(*major, version)),
        (_, Source::Zig) => Some(ctx.home.tool_dir(ZIG, version)),
        (Item::Component { id, .. }, Source::Github { .. }) => {
            Some(ctx.home.component_dir(id, version))
        }
        _ => None,
    }
}

/// How `item` at `release` unpacks, when its download is an archive (a
/// source, a JDK, zig, a macOS application).
pub fn unpacking(item: &Item, release: &Release, ctx: &Context) -> Option<Unpacking> {
    let kind = Kind::of(&asset(item, release)?.name)?;
    let dest = place(item, &release.version, ctx)?;
    let mut staging = dest.clone().into_os_string();
    staging.push(STAGING_SUFFIX);
    // A product's package keeps its folder (the .app); sources and tools are in one to drop.
    Some(Unpacking {
        kind,
        staging: staging.into(),
        strip: !matches!(item, Item::Product(_)),
    })
}

/// Moves what was unpacked into its place; where `components.ydata` says it is
/// (a JDK's home, a macOS application linked into ~/Applications).
pub fn finish(
    item: &Item,
    release: &Release,
    unpacked: &Unpacking,
    ctx: &Context,
) -> Result<PathBuf, String> {
    let dest = place(item, &release.version, ctx)
        .ok_or_else(|| format!("{}: nowhere to unpack", item.id()))?;
    files::remove_dir(&dest)?;
    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::rename(&unpacked.staging, &dest).map_err(|e| format!("{}: {e}", dest.display()))?;
    match item {
        Item::Product(p) => {
            let app = files::find_app(&dest)
                .ok_or_else(|| format!("{}: no .{} inside", p.id, launcher::APP_EXT))?;
            launcher::link_application(&app)?;
            Ok(app)
        }
        Item::Component { .. } => {
            let mac_jdk_home = dest.join("Contents").join("Home");
            Ok(if mac_jdk_home.is_dir() {
                mac_jdk_home
            } else {
                dest
            })
        }
    }
}

/// Installs `item` at `release` (`download`: its asset, already fetched);
/// where it went, for `components.ydata`.
pub fn install(
    item: &Item,
    release: &Release,
    download: Option<&Path>,
    ctx: &Context,
) -> Result<PathBuf, String> {
    let need = || download.ok_or_else(|| format!("{}: nothing downloaded to install", item.id()));
    if let Some(unpack) = unpacking(item, release, ctx) {
        files::remove_dir(&unpack.staging)?;
        archive::unpack(
            need()?,
            unpack.kind,
            &unpack.staging,
            unpack.strip,
            ctx.cancel,
        )?;
        return finish(item, release, &unpack, ctx);
    }
    let version = &release.version;
    match (item, item.source()) {
        (Item::Product(p), _) => files::product(
            p,
            version,
            need()?,
            asset(item, release).map(|a| a.name.as_str()),
            ctx,
        ),
        (_, Source::Crate { name }) => tools::cargo_install(item.id(), name, version, ctx),
        (_, Source::Rustup { toolchain }) => tools::toolchain(item.id(), toolchain, ctx),
        (_, Source::RustTargets { toolchain, targets }) => {
            tools::targets(item.id(), toolchain, targets, ctx)
        }
        _ => Err(format!(
            "{}: its download isn't an archive Yog Installer unpacks",
            item.id()
        )),
    }
}

/// Removes `item`'s `version` an update replaced (installed at `path`): its
/// files only — a product's desktop entry and links are the new version's now.
pub fn remove_replaced(
    item: &Item,
    version: &str,
    path: &Path,
    ctx: &Context,
) -> Result<(), String> {
    match item {
        Item::Product(p) => files::remove_dir(&ctx.home.product_dir(&p.id, version)),
        Item::Component { .. } => remove(item, version, path, ctx),
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
