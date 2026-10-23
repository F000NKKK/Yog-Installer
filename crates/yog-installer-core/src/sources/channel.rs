//! The channel: the newest catalog — the one built in, or the one the
//! latest Yog Installer release carries (cached), whichever is newer.

use crate::sources::catalog::Catalog;
use yog_home::Home;

/// The asset an Installer release carries its catalog in.
pub const CATALOG_ASSET: &str = "catalog.toml";

/// The newest catalog there is, refreshed from the channel when it can be
/// (else why not, for the notifications).
pub fn load(home: &Home) -> (Catalog, Option<String>) {
    let built_in = Catalog::built_in();
    let cached = home.cache_dir().join(CATALOG_ASSET);
    let problem = refresh(&built_in, &cached)
        .err()
        .map(|e| format!("The channel's catalog couldn't be refreshed: {e}"));
    let fresh = std::fs::read_to_string(&cached)
        .ok()
        .and_then(|t| Catalog::parse(&t).ok());
    let catalog = match fresh {
        Some(c) if c.revision > built_in.revision => c,
        _ => built_in,
    };
    (catalog, problem)
}

/// Fetches the latest release's catalog into `cached`, when it reads. No
/// release yet, or one without a catalog, isn't a problem: the built-in one
/// is the newest there is then.
fn refresh(built_in: &Catalog, cached: &std::path::Path) -> Result<(), String> {
    let Some(installer) = &built_in.installer else {
        return Ok(());
    };
    let Some(latest) = crate::sources::releases::of(&installer.source)?
        .into_iter()
        .next()
    else {
        return Ok(());
    };
    let Some(asset) = latest.asset(CATALOG_ASSET) else {
        return Ok(());
    };
    let text = crate::sources::http::get_text(&asset.url)?;
    Catalog::parse(&text)?;
    if let Some(dir) = cached.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(cached, text).map_err(|e| format!("{}: {e}", cached.display()))
}
