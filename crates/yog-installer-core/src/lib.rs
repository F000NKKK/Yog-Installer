//! Yog Installer's engine — everything but the window: the catalog (what
//! can be installed), the versions published, what's installed, what a
//! selection changes, whether it all fits together, and installing it.
//! See `docs/design.md`.

mod itself;
mod machine;
mod plans;
mod sources;

pub use itself::{selfinstall, selfupdate};
pub use machine::{detect, platform};
pub use plans::{compat, engine, overview, progress, selection};
pub use sources::{catalog, channel, download, http, latest, releases};

pub mod install;
