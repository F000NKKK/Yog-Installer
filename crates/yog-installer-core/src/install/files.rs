//! What's installed from a downloaded file: a product's package, a source
//! archive, a JDK, a tool — unpacked into Yog's home.

use super::launcher::{self, APP_EXT};
use super::{Context, run};
use crate::catalog::Product;
use crate::platform;
use std::path::{Path, PathBuf};

/// A Windows (NSIS) setup's switches: silent, into a folder (last, unquoted).
const NSIS_SILENT: &str = "/S";
const NSIS_DIR: &str = "/D=";
/// What an NSIS setup leaves to remove it.
const NSIS_UNINSTALL: &str = "uninstall.exe";

/// Installs a product's package that isn't an archive (macOS's is, see
/// `install::finish`): a Windows setup, a Linux AppImage; what starts it.
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
    // Linux: the AppImage itself, runnable, with a launcher entry.
    let program = dir.join(name);
    std::fs::copy(download, &program).map_err(|e| format!("{}: {e}", program.display()))?;
    make_executable(&program)?;
    launcher::add(&p.id, &p.name, &p.description, &program)?;
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
