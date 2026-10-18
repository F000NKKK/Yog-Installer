//! Where the OS lists what can be started: a Linux desktop entry, a link in
//! macOS's ~/Applications. (Windows' setups make their own shortcuts.)

use std::path::{Path, PathBuf};

/// A macOS application bundle's extension.
pub const APP_EXT: &str = "app";
/// Where macOS keeps a user's applications, under the home folder.
const MAC_APPLICATIONS: &str = "Applications";
/// Where Linux desktops look for a user's launcher entries.
const LINUX_APPLICATIONS: &str = ".local/share/applications";

/// The Linux launcher entry named `id`.
fn desktop_entry(id: &str) -> Option<PathBuf> {
    cfg!(target_os = "linux").then(|| {
        Some(
            dirs::home_dir()?
                .join(LINUX_APPLICATIONS)
                .join(format!("{id}.desktop")),
        )
    })?
}

/// Lists `program` as `name` among the desktop's applications (Linux).
pub fn add(id: &str, name: &str, comment: &str, program: &Path) -> Result<(), String> {
    let Some(entry) = desktop_entry(id) else {
        return Ok(());
    };
    if let Some(dir) = entry.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let text = format!(
        "[Desktop Entry]\nType=Application\nName={name}\nComment={comment}\nExec=\"{}\" %U\nTerminal=false\nCategories=Development;\n",
        program.display()
    );
    std::fs::write(&entry, text).map_err(|e| format!("{}: {e}", entry.display()))
}

pub fn remove(id: &str) {
    if let Some(entry) = desktop_entry(id) {
        let _ = std::fs::remove_file(entry);
    }
}

/// `app`'s link in ~/Applications, when it's a macOS application.
fn application_link(app: &Path) -> Option<PathBuf> {
    (app.extension()? == APP_EXT).then(|| {
        Some(
            dirs::home_dir()?
                .join(MAC_APPLICATIONS)
                .join(app.file_name()?),
        )
    })?
}

/// Links the macOS application `app` into ~/Applications.
pub fn link_application(app: &Path) -> Result<(), String> {
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

pub fn unlink_application(app: &Path) {
    if let Some(link) = application_link(app) {
        let _ = std::fs::remove_file(link);
    }
}
