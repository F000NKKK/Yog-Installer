//! Tauri's build with the Installer's plugin (its commands and default
//! permission, from `src/host/commands.rs`), and the window's copy of the
//! commands and events (`src/host.generated.ts`).

use std::path::Path;

use tauri_build::{Attributes, DefaultPermissionRule, InlinedPlugin};
use yog_codegen::{Header, Regenerate};
use yog_codegen_typescript::TypeScript;

#[path = "src/host/commands.rs"]
mod commands;
#[path = "src/host/events.rs"]
mod events;

/// Where the window's copy goes, from this crate.
const HOST_TS: &str = "../src/host.generated.ts";

fn main() {
    let header = Header {
        tool: "src-tauri/build.rs",
        source: "src-tauri/src/host/",
        regenerate: Regenerate::EveryBuild,
    };
    let text = format!(
        "{}\n{}\n{}",
        header.comment::<TypeScript>(),
        substrate_host_contract::commands::typescript(
            "installer",
            "The Installer's commands: `installer.installer.<command>`, for `invokeCommand`.",
            commands::ALL
        ),
        substrate_host_contract::events::typescript(
            "events",
            "The events the Installer's backend sends its window.",
            events::ALL
        ),
    );
    yog_codegen::write_if_changed(Path::new(HOST_TS), text).expect("src/ is writable");
    let attributes = commands::ALL
        .iter()
        .fold(Attributes::new(), |attributes, domain| {
            attributes.plugin(
                domain.plugin,
                InlinedPlugin::new()
                    .commands(domain.commands)
                    .default_permission(DefaultPermissionRule::AllowAllCommands),
            )
        });
    tauri_build::try_build(attributes).expect("tauri build");
}
