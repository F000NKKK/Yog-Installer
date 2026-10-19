//! The Installer's commands — the window calls them as `installer.installer.<command>`.

use substrate_platform::commands::Domain;

pub const INSTALLER: Domain = Domain {
    key: "installer",
    plugin: "yog-installer",
    commands: &[
        "snapshot",
        "selection_of",
        "plan",
        "apply",
        "uninstall",
        "repair",
        "resume",
        "forget_pending",
        "cancel",
        "pause",
        "launch",
        "self_update",
    ],
};

/// Every domain.
pub const ALL: &[&Domain] = &[&INSTALLER];
