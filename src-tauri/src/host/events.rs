//! The events the Installer's backend sends its window.

/// An operation's progress: `{ product, event }`.
pub const PROGRESS: &str = "installer-progress";

/// The Installer's own update downloading: `{ done, total }`.
pub const SELF_UPDATE: &str = "installer-self-update";

/// Every event, for the window's copy: (name in code, event).
pub const ALL: &[(&str, &str)] = &[("progress", PROGRESS), ("selfUpdate", SELF_UPDATE)];
