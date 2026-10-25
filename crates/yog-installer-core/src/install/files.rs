//! What's installed from a downloaded file: a product's package, a source
//! archive, a JDK, a tool — unpacked into Yog's home.

use super::launcher::{self, APP_EXT};
use super::{Context, run};
use crate::machine::platform;
use crate::sources::catalog::Product;
use std::path::{Path, PathBuf};

/// A Windows (NSIS) setup's switches: silent, into a folder (last, unquoted).
const NSIS_SILENT: &str = "/S";
const NSIS_DIR: &str = "/D=";
/// What an NSIS setup leaves to remove it.
const NSIS_UNINSTALL: &str = "uninstall.exe";
/// A Debian package's name ends so; its program is in `usr/bin/`.
const DEB_EXT: &str = ".deb";
const DEB_BIN: [&str; 2] = ["usr", "bin"];

/// Installs a product's package that isn't an archive (macOS's is, see
/// `install::finish`): a Windows setup, a Linux AppImage or Debian
/// package (unpacked, not installed); what starts it.
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
    if cfg!(windows) {
        let args = [
            NSIS_SILENT.to_string(),
            format!("{NSIS_DIR}{}", dir.display()),
        ];
        run::tool(download, &args, ctx.cancel, &ctx.output(&p.id))?;
        return Ok(dir.join(platform::exe(&p.program)));
    }
    // Linux: the AppImage itself, runnable, or the Debian package's
    // program; with a launcher entry.
    let program = if name.ends_with(DEB_EXT) {
        super::deb::unpack(download, &dir, ctx.cancel)?;
        DEB_BIN
            .iter()
            .fold(dir.clone(), |d, part| d.join(part))
            .join(&p.program)
    } else {
        let program = dir.join(name);
        std::fs::copy(download, &program).map_err(|e| format!("{}: {e}", program.display()))?;
        program
    };
    make_executable(&program)?;
    // Its icon for the desktop entry; an entry without one is still fine.
    let icon = p
        .icon
        .as_deref()
        .and_then(|url| crate::sources::http::get_bytes(url).ok())
        .and_then(|png| {
            let file = dir.join(launcher::ICON_FILE);
            std::fs::write(&file, png).ok().map(|_| file)
        });
    launcher::add(&p.id, &p.name, &p.description, &program, icon.as_deref())?;
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
    launcher::unlink_application(program);
    launcher::remove(&p.id);
    remove_dir(dir)
}

/// Every version folder beside `keep` (a product's version folder) removed.
pub fn remove_other_versions(keep: &Path) -> Result<(), String> {
    let Some(Ok(entries)) = keep.parent().map(std::fs::read_dir) else {
        return Ok(());
    };
    for dir in entries.flatten().map(|e| e.path()) {
        if dir.is_dir() && dir != keep {
            remove_dir(&dir)?;
        }
    }
    Ok(())
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

pub(crate) fn find_app(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|x| x == APP_EXT))
}

pub(crate) fn make_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}
