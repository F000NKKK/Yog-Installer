//! What the Installer holds between commands: Yog's home, the catalog, the
//! newest releases last asked for, and the operation running (to cancel it).

use std::collections::BTreeMap;
use std::sync::Mutex;
use yog_home::Home;
use yog_installer_core::catalog::Catalog;
use yog_installer_core::detect::{self, Found, RealSystem};
use yog_installer_core::progress::Cancel;
use yog_installer_core::releases::Release;

pub struct AppState {
    /// Yog's home; why there's none (no home folder), shown by the window.
    home: Result<Home, String>,
    pub catalog: Mutex<Catalog>,
    /// Id → its newest release; empty until first asked.
    pub latest: Mutex<BTreeMap<String, Release>>,
    /// What couldn't be asked or refreshed, for the notifications.
    pub problems: Mutex<Vec<String>>,
    /// The operation running, if one is.
    pub running: Mutex<Option<Cancel>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            home: Home::locate(),
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
        self.catalog.lock().expect("catalog lock").clone()
    }

    pub fn latest(&self) -> BTreeMap<String, Release> {
        self.latest.lock().expect("latest lock").clone()
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
        let (latest, mut problems) = yog_installer_core::overview::latest(&catalog);
        problems.extend(problem);
        *self.catalog.lock().expect("catalog lock") = catalog;
        *self.latest.lock().expect("latest lock") = latest;
        *self.problems.lock().expect("problems lock") = problems;
        Ok(())
    }

    /// Starts an operation, unless one is running; its cancel.
    pub fn start(&self) -> Result<Cancel, String> {
        let mut running = self.running.lock().expect("running lock");
        if running.is_some() {
            return Err("Another operation is running.".into());
        }
        let cancel = Cancel::default();
        *running = Some(cancel.clone());
        Ok(cancel)
    }

    pub fn finish(&self) {
        *self.running.lock().expect("running lock") = None;
    }
}
