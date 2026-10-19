//! What the Installer's window and backend share by name: its commands (a
//! domain, Substrate's infrastructure) and the events it sends. `build.rs`
//! includes these files for the permissions and the window's copy
//! (`src/host.generated.ts`).

pub mod commands;
pub mod events;
