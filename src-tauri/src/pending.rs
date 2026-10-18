//! The install or modify running, written down while it runs: when the
//! Installer stops before it ends (a crash, a power cut), the next start
//! offers to go on with it — what's installed by then is in
//! `components.json`, and downloads resume where they stopped.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use yog_home::Home;
use yog_installer_core::engine::Mode;
use yog_installer_core::selection::Selection;

/// The journal, in the Installer's cache.
const FILE: &str = "pending.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    serde_json::from_str(&std::fs::read_to_string(path(home)).ok()?).ok()
}

pub fn write(home: &Home, pending: &Pending) -> Result<(), String> {
    let file = path(home);
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let text = serde_json::to_string(pending).map_err(|e| e.to_string())?;
    std::fs::write(&file, text).map_err(|e| format!("{}: {e}", file.display()))
}

pub fn clear(home: &Home) {
    let _ = std::fs::remove_file(path(home));
}
