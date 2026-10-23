//! Yog Installer putting itself in its place, as Visual Studio Installer
//! does: started from anywhere (a download), it copies itself where it
//! lives — Yog's home on Linux (with a desktop entry), ~/Applications on
//! macOS; Windows' setup already put it there — records itself in
//! `components.ydata`, and is started again from there. Its updates replace
//! that copy (`selfupdate`).

use crate::install::launcher::{self, APP_EXT};
use std::path::{Path, PathBuf};
use yog_home::Home;
use yog_home::components::id::INSTALLER;
use yog_home::components::{Components, Installed};

/// The variable an AppImage is told its own path in.
pub const APPIMAGE_ENV: &str = "APPIMAGE";
/// The installed AppImage's extension.
const APPIMAGE_EXT: &str = "AppImage";
/// What macOS copies an application bundle with.
const DITTO: &str = "ditto";
/// What the desktop entry says it is.
const COMMENT: &str =
    "Installs and updates Yog IDLE, the Yog CLI, the loader SDK, JDKs and the Rust toolchain.";

/// Where the running Installer is now.
pub enum Placed {
    /// In its place.
    Here,
    /// Copied into its place: start that one and quit.
    Moved(PathBuf),
}

/// Puts the running Installer (`name`, `program`, at `version`, its `icon` a PNG) in its place.
pub fn ensure(
    home: &Home,
    name: &str,
    program: &str,
    version: &str,
    icon: &[u8],
) -> Result<Placed, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    if let Some(app) = exe
        .ancestors()
        .find(|p| p.extension().is_some_and(|x| x == APP_EXT))
    {
        return place_app(home, app, version);
    }
    let Some(image) = std::env::var_os(APPIMAGE_ENV).map(PathBuf::from) else {
        // Windows (its setup chose the place), or a program not packaged at all.
        record(home, version, exe)?;
        return Ok(Placed::Here);
    };
    let dest = home
        .installer_dir()
        .join(format!("{program}.{APPIMAGE_EXT}"));
    let moved = image != dest;
    if moved {
        std::fs::create_dir_all(home.installer_dir()).map_err(|e| e.to_string())?;
        let staged = dest.with_extension("new");
        std::fs::copy(&image, &staged).map_err(|e| format!("{}: {e}", staged.display()))?;
        crate::install::make_executable(&staged)?;
        std::fs::rename(&staged, &dest).map_err(|e| format!("{}: {e}", dest.display()))?;
    }
    let icon_file = home.installer_dir().join(launcher::ICON_FILE);
    std::fs::write(&icon_file, icon).map_err(|e| format!("{}: {e}", icon_file.display()))?;
    launcher::add(INSTALLER, name, COMMENT, &dest, Some(&icon_file))?;
    record(home, version, dest.clone())?;
    Ok(if moved {
        Placed::Moved(dest)
    } else {
        Placed::Here
    })
}

/// macOS: the bundle, copied into ~/Applications.
fn place_app(home: &Home, app: &Path, version: &str) -> Result<Placed, String> {
    let dir = dirs::home_dir()
        .ok_or("no home folder")?
        .join("Applications");
    let dest = dir.join(app.file_name().ok_or("the .app has no name")?);
    if app == dest {
        record(home, version, dest)?;
        return Ok(Placed::Here);
    }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(&dest);
    let status = substrate_platform::process::command(DITTO)
        .arg(app)
        .arg(&dest)
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!(
            "copying {} into {}: {status}",
            app.display(),
            dir.display()
        ));
    }
    record(home, version, dest.clone())?;
    Ok(Placed::Moved(dest))
}

/// Records the Installer at `path` in `components.ydata` (for Yog IDLE's Tools menu).
fn record(home: &Home, version: &str, path: PathBuf) -> Result<(), String> {
    let file = home.components_file();
    let mut components = Components::read(&file)?;
    components.installed.retain(|c| c.id != INSTALLER);
    components.put(Installed {
        id: INSTALLER.into(),
        version: version.into(),
        path,
    });
    components.write(&file)
}
