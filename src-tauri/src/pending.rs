//! The install or modify running, written down while it runs: when the
//! Installer stops before it ends (a crash, a power cut), the next start
//! offers to go on with it — what's installed by then is in
//! `components.ydata`, and downloads resume where they stopped.

use std::path::PathBuf;
use yog_home::Home;
use yog_installer_core::engine::Mode;
use yog_installer_core::selection::Selection;

/// The journal, in the Installer's cache.
const FILE: &str = yog_ids::concatcp!("pending", yog_ids::sep::DOT, yog_ids::Format::Data.ext());

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, yog_data::Schema)]
#[serde(rename_all = "camelCase")]
pub struct Pending {
    pub product: String,
    pub selection: Selection,
    pub mode: Mode,
    pub keep_downloads: bool,
}

fn path(home: &Home) -> PathBuf {
    home.cache_dir().join(FILE)
}

pub fn read(home: &Home) -> Option<Pending> {
    yog_data::read_bytes(&std::fs::read(path(home)).ok()?).ok()
}

pub fn write(home: &Home, pending: &Pending) -> Result<(), String> {
    let file = path(home);
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(&file, yog_data::to_bytes(pending))
        .map_err(|e| format!("{}: {e}", file.display()))
}

pub fn clear(home: &Home) {
    let _ = std::fs::remove_file(path(home));
}
