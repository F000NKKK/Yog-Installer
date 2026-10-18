//! Yog Installer's engine — everything but the window: the catalog (what
//! can be installed), the versions published, what's installed, what a
//! selection changes, whether it all fits together, and installing it.
//! See `docs/design.md`.

pub mod catalog;
pub mod channel;
pub mod compat;
pub mod detect;
pub mod engine;
pub mod http;
pub mod install;
pub mod overview;
pub mod platform;
pub mod progress;
pub mod releases;
pub mod selection;
pub mod selfupdate;
