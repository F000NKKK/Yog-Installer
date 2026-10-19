//! What the Installer holds between commands: Yog's home, the catalog, the
//! newest releases last asked for, and the operation running (to cancel it).

use substrate_platform::sync::Locked;
use std::collections::BTreeMap;
use std::sync::Mutex;
use yog_home::Home;
use yog_installer_core::catalog::Catalog;
use yog_installer_core::detect::{self, Found, RealSystem};
use yog_installer_core::latest::News;
use yog_installer_core::progress::Cancel;
use yog_installer_core::releases::Release;

pub struct AppState {
    /// Yog's home; why there's none (no home folder), shown by the window.
    home: Result<Home, String>,
    pub catalog: Mutex<Catalog>,
    /// Id → its newest release; empty until first asked.
    pub latest: Mutex<BTreeMap<String, Release>>,
    /// The latest releases of Yog's repos, newest first.
    pub news: Mutex<Vec<News>>,
    /// What went wrong once (at start), kept through refreshes.
    pub notices: Mutex<Vec<String>>,
    /// What couldn't be asked or refreshed, for the notifications.
    pub problems: Mutex<Vec<String>>,
    /// The operation running, if one is.
    pub running: Mutex<Option<Cancel>>,
}

impl AppState {
    pub fn new() -> Self {
        let home = Home::locate();
        if let Ok(home) = &home {
            yog_installer_core::http::set_cache(home.cache_dir());
        }
        Self {
            home,
            news: Mutex::new(Vec::new()),
            notices: Mutex::new(Vec::new()),
            catalog: Mutex::new(Catalog::built_in()),
            latest: Mutex::new(BTreeMap::new()),
            problems: Mutex::new(Vec::new()),
            running: Mutex::new(None),
        }
    }

    pub fn home(&self) -> Result<&Home, String> {
        self.home.as_ref().map_err(Clone::clone)
    }

    pub fn catalog(&self) -> Catalog {
        self.catalog.locked().clone()
    }

    pub fn latest(&self) -> BTreeMap<String, Release> {
        self.latest.locked().clone()
    }

    /// Everything installed, by the Installer or before it.
    pub fn installed(&self) -> Result<Vec<Found>, String> {
        Ok(detect::installed(
            &self.catalog(),
            &self.home()?.components()?,
            &RealSystem,
        ))
    }

    /// Asks the channel and every source again.
    pub fn refresh(&self) -> Result<(), String> {
        let (catalog, problem) = yog_installer_core::channel::load(self.home()?);
        let mut latest = yog_installer_core::latest::fetch(&catalog);
        latest.problems.extend(problem);
        *self.catalog.locked() = catalog;
        *self.latest.locked() = latest.newest;
        *self.news.locked() = latest.news;
        *self.problems.locked() = latest.problems;
        Ok(())
    }

    /// Starts an operation, unless one is running; its cancel.
    pub fn start(&self) -> Result<Cancel, String> {
        let mut running = self.running.locked();
        if running.is_some() {
            return Err("Another operation is running.".into());
        }
        let cancel = Cancel::default();
        *running = Some(cancel.clone());
        Ok(cancel)
    }

    pub fn finish(&self) {
        *self.running.locked() = None;
    }
}
