//! Carrying out a plan: each change downloaded, installed and recorded in
//! `components.ydata`, with progress events, until it's done, fails or is cancelled.

use crate::install::{self, Context, Item, archive};
use crate::plans::progress::{CANCELLED, Event};
use crate::plans::selection::Change;
use crate::sources::catalog::{Catalog, Source};
use crate::sources::releases::Release;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use yog_home::components::{Components, Installed};

/// When downloads happen, as VS Installer offers it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, yog_data::Schema)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    /// Each package installs as soon as it's downloaded.
    #[default]
    InstallWhileDownloading,
    /// Everything downloads first, then installs (nothing changes if a download fails).
    DownloadAllThenInstall,
}

/// What to do: the changes, and the release each install or update takes.
pub struct Plan {
    pub changes: Vec<Change>,
    pub releases: BTreeMap<String, Release>,
}

impl Plan {
    /// Bytes to download, as far as the releases say.
    pub fn download_size(&self, catalog: &Catalog) -> u64 {
        self.changes
            .iter()
            .filter_map(|c| self.asset_size(catalog, c))
            .sum()
    }

    fn asset_size(&self, catalog: &Catalog, change: &Change) -> Option<u64> {
        let release = self.releases.get(change.id())?;
        install::asset(&item(catalog, change.id())?, release).map(|a| a.size)
    }
}

/// The catalog's product or component `id`.
pub fn item<'c>(catalog: &'c Catalog, id: &'c str) -> Option<Item<'c>> {
    if let Some(p) = catalog.product(id) {
        return Some(Item::Product(p));
    }
    catalog.component(id).map(|c| Item::Component {
        id: &c.id,
        source: &c.source,
    })
}

/// Carries out `plan`; `keep_downloads`: leave what was downloaded in the cache.
pub fn apply(
    catalog: &Catalog,
    plan: &Plan,
    mode: Mode,
    keep_downloads: bool,
    ctx: &Context,
) -> Result<(), String> {
    ctx.sink.event(Event::Started {
        packages: plan.changes.len(),
        bytes: plan.download_size(catalog),
    });
    let result = run(catalog, plan, mode, keep_downloads, ctx);
    let cancelled = result.as_ref().is_err_and(|e| e == CANCELLED);
    ctx.sink.event(Event::Finished {
        ok: result.is_ok(),
        cancelled,
        error: result.clone().err().filter(|_| !cancelled),
    });
    result
}

fn run(
    catalog: &Catalog,
    plan: &Plan,
    mode: Mode,
    keep_downloads: bool,
    ctx: &Context,
) -> Result<(), String> {
    let mut downloads: BTreeMap<String, PathBuf> = BTreeMap::new();
    if mode == Mode::DownloadAllThenInstall {
        for change in &plan.changes {
            if let Some(path) = download(catalog, plan, change, ctx)? {
                downloads.insert(change.id().to_string(), path);
            }
        }
    }
    let file = ctx.home.components_file();
    let mut components = Components::read(&file)?;
    for (i, change) in plan.changes.iter().enumerate() {
        ctx.cancel.check()?;
        let id = change.id();
        let the = item(catalog, id).ok_or_else(|| format!("{id} isn't in the catalog"))?;
        let installing = Event::Installing {
            component: id.to_string(),
            index: i + 1,
            of: plan.changes.len(),
        };
        let release = plan.releases.get(id);
        // An archive, while it downloads: unpacked as the bytes arrive.
        let streamed = match (mode, release, downloads.contains_key(id)) {
            (Mode::InstallWhileDownloading, Some(release), false) => {
                stream_in(&the, release, &installing, keep_downloads, ctx)?
            }
            _ => None,
        };
        let download = match (&streamed, downloads.remove(id)) {
            (Some(_), _) => None,
            (None, Some(path)) => Some(path),
            (None, None) => download(catalog, plan, change, ctx)?,
        };
        if streamed.is_none() {
            ctx.sink.event(installing);
        }
        match change {
            Change::Install { version, .. } | Change::Update { to: version, .. } => {
                let release = release.ok_or_else(|| format!("{id}: no release {version}"))?;
                let path = match streamed {
                    Some(path) => path,
                    None => install::install(&the, release, download.as_deref(), ctx)?,
                };
                if let Change::Update { from, .. } = change {
                    replace_old(&the, &mut components, (from, version), &path, ctx)?;
                }
                components.put(Installed {
                    id: id.to_string(),
                    version: version.clone(),
                    path,
                });
                ctx.sink.event(Event::Installed {
                    component: id.to_string(),
                    version: version.clone(),
                });
            }
            Change::Remove { version, .. } => {
                let path = components
                    .installed
                    .iter()
                    .find(|c| c.id == id && &c.version == version)
                    .map(|c| c.path.clone());
                install::remove(&the, version, &path.unwrap_or_default(), ctx)?;
                components.remove(id, version);
                ctx.sink.event(Event::Removed {
                    component: id.to_string(),
                });
            }
        }
        components.write(&file)?;
        if let Some(path) = download.filter(|_| !keep_downloads) {
            let _ = std::fs::remove_file(path);
        }
    }
    Ok(())
}

/// After an update `from` → `to`: the old version's files go (unless the
/// new one is in the same place — a tool replaced in place), and its record;
/// a product's every other version too (one an older update left behind).
fn replace_old(
    the: &Item,
    components: &mut Components,
    (from, to): (&str, &str),
    new: &PathBuf,
    ctx: &Context,
) -> Result<(), String> {
    let id = the.id();
    let old = components
        .installed
        .iter()
        .find(|c| c.id == id && c.version == from)
        .map(|c| c.path.clone());
    if let Some(old) = old.filter(|o| o != new && !in_place(the.source())) {
        install::remove_replaced(the, from, &old, ctx)?;
    }
    components.remove(id, from);
    install::remove_other_versions(the, to, ctx)
}

/// rustup and cargo keep one version of what they install: an update replaces it.
fn in_place(source: &Source) -> bool {
    matches!(
        source,
        Source::Crate { .. } | Source::Rustup { .. } | Source::RustTargets { .. }
    )
}

/// Downloads `the`'s archive and unpacks it at once, while it arrives
/// (`installing` is told as it starts); where it went — none when its
/// download isn't an archive (it's installed from the file).
fn stream_in(
    the: &Item,
    release: &Release,
    installing: &Event,
    keep_downloads: bool,
    ctx: &Context,
) -> Result<Option<PathBuf>, String> {
    let (Some(asset), Some(unpack)) = (
        install::asset(the, release),
        install::unpacking(the, release, ctx),
    ) else {
        return Ok(None);
    };
    let dest = cache_path(ctx, the.id(), release, &asset.name);
    let _ = std::fs::remove_dir_all(&unpack.staging);
    ctx.sink.event(installing.clone());
    let progress = |done, total| {
        ctx.sink.event(Event::Downloading {
            component: the.id().to_string(),
            done,
            total,
        })
    };
    let unpack_into =
        |pipe| archive::unpack_stream(pipe, unpack.kind, &unpack.staging, unpack.strip, ctx.cancel);
    crate::sources::download::stream(
        &asset.url,
        &dest,
        asset.sha256.as_deref(),
        ctx.cancel,
        &progress,
        Some(unpack_into),
    )?;
    let path = install::finish(the, release, &unpack, ctx)?;
    if !keep_downloads {
        let _ = std::fs::remove_file(&dest);
    }
    Ok(Some(path))
}

/// Where `id`'s download of `release` waits in the cache.
fn cache_path(ctx: &Context, id: &str, release: &Release, name: &str) -> PathBuf {
    ctx.home
        .cache_dir()
        .join(id)
        .join(&release.version)
        .join(name)
}

/// Fetches `change`'s asset into the cache; none when it has none (a tool's, a removal).
fn download(
    catalog: &Catalog,
    plan: &Plan,
    change: &Change,
    ctx: &Context,
) -> Result<Option<PathBuf>, String> {
    let (Change::Install { .. } | Change::Update { .. }) = change else {
        return Ok(None);
    };
    let id = change.id();
    let Some(release) = plan.releases.get(id) else {
        return Ok(None);
    };
    let Some(asset) = item(catalog, id).and_then(|i| install::asset(&i, release).cloned()) else {
        return Ok(None);
    };
    let dest = cache_path(ctx, id, release, &asset.name);
    let progress = |done, total| {
        ctx.sink.event(Event::Downloading {
            component: id.to_string(),
            done,
            total,
        })
    };
    crate::sources::download::download(
        &asset.url,
        &dest,
        asset.sha256.as_deref(),
        ctx.cancel,
        &progress,
    )?;
    Ok(Some(dest))
}
