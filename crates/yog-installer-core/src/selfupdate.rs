//! Yog Installer updating itself — first, before anything else, as Visual
//! Studio Installer does. The new version replaces the running one, then
//! the Installer starts again.

use crate::catalog::Catalog;
use crate::install::archive::{self, Kind};
use crate::progress::Cancel;
use crate::releases::Release;
use crate::selfinstall::APPIMAGE_ENV;
use crate::{compat, platform};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use yog_home::components::id::INSTALLER;

/// A Windows (NSIS) setup's silent switch.
const NSIS_SILENT: &str = "/S";

/// The newest Installer in `newest` (`latest::fetch`), if it's newer than `current`.
pub fn newer<'r>(newest: &'r BTreeMap<String, Release>, current: &str) -> Option<&'r Release> {
    newest
        .get(INSTALLER)
        .filter(|r| compat::compare(&r.version, current).is_gt())
}

/// What to do once the new version is in place.
pub enum Then {
    /// Start this program (the new Installer) and quit.
    Restart(PathBuf),
    /// Quit: a setup that's now running replaces the Installer and starts it.
    Quit,
}

/// Puts `release` in place of the running Installer (`cache`: where to download).
pub fn apply(
    catalog: &Catalog,
    release: &Release,
    cache: &Path,
    cancel: &Cancel,
    progress: &dyn Fn(u64, Option<u64>),
) -> Result<Then, String> {
    let installer = catalog
        .installer
        .as_ref()
        .ok_or("no Installer in the catalog")?;
    let crate::catalog::Source::Github { assets, .. } = &installer.source else {
        return Err("the Installer isn't a GitHub release".into());
    };
    let suffix = assets
        .get(&platform::tag())
        .ok_or("no Installer for this platform")?;
    let asset = release
        .asset_ending(suffix)
        .ok_or("the release has no Installer for this platform")?;
    let download = cache.join(&asset.name);
    crate::download::download(
        &asset.url,
        &download,
        asset.sha256.as_deref(),
        cancel,
        progress,
    )?;
    if let Some(kind) = Kind::of(&asset.name) {
        return replace_app(&download, kind, cancel);
    }
    if cfg!(windows) {
        crate::platform::command(&download)
            .arg(NSIS_SILENT)
            .spawn()
            .map_err(|e| format!("{}: {e}", download.display()))?;
        return Ok(Then::Quit);
    }
    replace_appimage(&download)
}

/// Linux: the running AppImage's file, swapped for the new one.
fn replace_appimage(new: &Path) -> Result<Then, String> {
    let current = std::env::var_os(APPIMAGE_ENV)
        .map(PathBuf::from)
        .ok_or("this Installer isn't running from an AppImage — update it by hand")?;
    let staged = current.with_extension("new");
    std::fs::copy(new, &staged).map_err(|e| format!("{}: {e}", staged.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
    }
    std::fs::rename(&staged, &current).map_err(|e| format!("{}: {e}", current.display()))?;
    Ok(Then::Restart(current))
}

/// macOS: the running `.app` bundle, swapped for the new one.
fn replace_app(archive_path: &Path, kind: Kind, cancel: &Cancel) -> Result<Then, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let app = exe
        .ancestors()
        .find(|p| p.extension().is_some_and(|x| x == "app"))
        .ok_or("this Installer isn't running from an .app")?
        .to_path_buf();
    let parent = app.parent().ok_or("the .app has no folder")?;
    let staging = parent.join(".yog-installer-update");
    let _ = std::fs::remove_dir_all(&staging);
    archive::unpack(archive_path, kind, &staging, false, cancel)?;
    let new_app = std::fs::read_dir(&staging)
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|x| x == "app"))
        .ok_or("the update has no .app")?;
    let old = app.with_extension("old");
    let _ = std::fs::remove_dir_all(&old);
    std::fs::rename(&app, &old).map_err(|e| format!("{}: {e}", app.display()))?;
    std::fs::rename(&new_app, &app).map_err(|e| format!("{}: {e}", app.display()))?;
    let _ = std::fs::remove_dir_all(&old);
    let _ = std::fs::remove_dir_all(&staging);
    Ok(Then::Restart(app))
}
