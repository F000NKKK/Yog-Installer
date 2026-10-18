# Yog Installer

Installs, updates and removes everything a Yog developer needs — the way
Visual Studio Installer does for Visual Studio.

- **Products**: Yog IDLE — install, update, modify, repair, uninstall, launch.
- **Workloads**: *Mod development* (the Yog CLI, the Rust toolchain and the
  five targets a `.yog` carries, zig, cargo-zigbuild, a JDK, the loader SDK)
  and *Loader development* (the loader's source, every JDK its Minecraft
  versions need).
- **Individual components** with live versions: GitHub releases, crates.io,
  Adoptium, ziglang.org.
- **Compatibility**: a release's `yog-release.json` says what it needs of
  the others; a selection that doesn't fit is stopped before it installs.
- **Updates itself** first, then the rest.

Everything goes under Yog's home (`~/.yog`, or `$YOG_HOME`) and is listed in
`~/.yog/components.json`, which `yog run` and Yog IDLE read. A machine's own
overrides — a loader checkout of its own, its JDKs, its graphics card's
environment — go in `~/.yog/local.toml`, which the Installer never touches.

## Layout

| Path | What |
|---|---|
| `catalog/catalog.toml` | The channel: products, workloads, components, where each is published |
| `crates/yog-installer-core` | The engine: catalog, versions, detection, selections, compatibility, installs, self-update |
| `src-tauri/` | The app's commands over the engine |
| `src/` | The window (React, on Substrate-Platform's UI) |

See [docs/design.md](docs/design.md).

## Building

Substrate-Platform and Yog-Mod-Loader are checked out next to this repo.

```sh
npm install
npm run tauri dev      # the window, live
cargo test --workspace # the engine (network checks: -- --ignored)
```

## License

AGPL-3.0-only.
