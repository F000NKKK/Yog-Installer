//! What's installed from a downloaded file: a product's package, a source
//! archive, a JDK, a tool — unpacked into Yog's home.

use super::archive::{self, Kind};
use super::{Context, run};
use crate::catalog::Product;
use crate::platform;
use std::path::{Path, PathBuf};

/// A macOS application bundle's extension.
const APP_EXT: &str = "app";
/// Where macOS keeps a user's applications, under the home folder.
const MAC_APPLICATIONS: &str = "Applications";
/// Where Linux desktops look for a user's launcher entries.
const LINUX_APPLICATIONS: &str = ".local/share/applications";
/// A Windows (NSIS) setup's switches: silent, into a folder (last, unquoted).
const NSIS_SILENT: &str = "/S";
const NSIS_DIR: &str = "/D=";
/// What an NSIS setup leaves to remove it.
const NSIS_UNINSTALL: &str = "uninstall.exe";

/// Unpacks the archive `download` (named `name`) into `dest`, without its top folder.
pub fn unpacked(
    download: &Path,
    name: &str,
    dest: &Path,
    ctx: &Context,
) -> Result<PathBuf, String> {
    let kind =
        Kind::of(name).ok_or_else(|| format!("{name}: not an archive Yog Installer unpacks"))?;
    remove_dir(dest)?;
    archive::unpack(download, kind, dest, true, ctx.cancel)?;
    Ok(dest.to_path_buf())
}

/// A JDK unpacked into `dest`; its home (macOS's is `Contents/Home`).
pub fn jdk(
    download: &Path,
    asset: Option<&crate::releases::Asset>,
    dest: &Path,
    ctx: &Context,
) -> Result<PathBuf, String> {
    let name = asset.map_or("", |a| a.name.as_str());
    let dir = unpacked(download, name, dest, ctx)?;
    let mac_home = dir.join("Contents").join("Home");
    Ok(if mac_home.is_dir() { mac_home } else { dir })
}

/// Installs a product's package for this platform; what starts it.
pub fn product(
    p: &Product,
    version: &str,
    download: &Path,
    asset: Option<&str>,
    ctx: &Context,
) -> Result<PathBuf, String> {
    let dir = ctx.home.product_dir(&p.id, version);
    let name = asset.unwrap_or_default();
    remove_dir(&dir)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    if let Some(kind) = Kind::of(name) {
        // macOS: the .app, unpacked, and linked into ~/Applications.
        archive::unpack(download, kind, &dir, false, ctx.cancel)?;
        let app = find_app(&dir).ok_or_else(|| format!("{name}: no .{APP_EXT} inside"))?;
        link_application(&app)?;
        return Ok(app);
    }
    if cfg!(windows) {
        let args = [
            NSIS_SILENT.to_string(),
            format!("{NSIS_DIR}{}", dir.display()),
        ];
        run::tool(download, &args, ctx.cancel, &ctx.output(&p.id))?;
        return Ok(dir.join(platform::exe(&p.program)));
    }
    // Linux: the AppImage itself, runnable, with a launcher entry.
    let program = dir.join(name);
    std::fs::copy(download, &program).map_err(|e| format!("{}: {e}", program.display()))?;
    make_executable(&program)?;
    write_desktop_entry(p, &program)?;
    Ok(program)
}

/// Removes a product installed in `dir`, started by `program`.
pub fn remove_product(
    p: &Product,
    dir: &Path,
    program: &Path,
    ctx: &Context,
) -> Result<(), String> {
    let uninstall = dir.join(NSIS_UNINSTALL);
    if cfg!(windows) && uninstall.is_file() {
        run::tool(
            &uninstall,
            &[NSIS_SILENT.to_string()],
            ctx.cancel,
            &ctx.output(&p.id),
        )?;
    }
    if let Some(link) = application_link(program) {
        let _ = std::fs::remove_file(link);
    }
    if let Some(entry) = desktop_entry(p) {
        let _ = std::fs::remove_file(entry);
    }
    remove_dir(dir)
}

/// Removes `dir` and all in it, when it's there.
pub fn remove_dir(dir: &Path) -> Result<(), String> {
    match std::fs::remove_dir_all(dir) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            Err(format!("{}: {e}", dir.display()))
        }
        _ => Ok(()),
    }
}

fn find_app(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|x| x == APP_EXT))
}

fn application_link(app: &Path) -> Option<PathBuf> {
    (app.extension()? == APP_EXT).then(|| {
        Some(
            dirs::home_dir()?
                .join(MAC_APPLICATIONS)
                .join(app.file_name()?),
        )
    })?
}

fn link_application(app: &Path) -> Result<(), String> {
    #[cfg(unix)]
    if let Some(link) = application_link(app) {
        let _ = std::fs::remove_file(&link);
        if let Some(dir) = link.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::os::unix::fs::symlink(app, &link).map_err(|e| format!("{}: {e}", link.display()))?;
    }
    Ok(())
}

fn make_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

/// The Linux launcher entry of `p`.
fn desktop_entry(p: &Product) -> Option<PathBuf> {
    cfg!(target_os = "linux").then(|| {
        Some(
            dirs::home_dir()?
                .join(LINUX_APPLICATIONS)
                .join(format!("{}.desktop", p.id)),
        )
    })?
}

fn write_desktop_entry(p: &Product, program: &Path) -> Result<(), String> {
    let Some(entry) = desktop_entry(p) else {
        return Ok(());
    };
    if let Some(dir) = entry.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let text = format!(
        "[Desktop Entry]\nType=Application\nName={}\nComment={}\nExec=\"{}\" %U\nTerminal=false\nCategories=Development;\n",
        p.name,
        p.description,
        program.display()
    );
    std::fs::write(&entry, text).map_err(|e| format!("{}: {e}", entry.display()))
}
