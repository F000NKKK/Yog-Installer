# Yog Installer — design

Yog Installer installs, updates and removes everything a Yog developer
needs, the way Visual Studio Installer does for Visual Studio: products with
workloads and individual components, live versions, one place that updates
the editor and itself.

## What it manages

| Kind | Examples | Where it goes |
|---|---|---|
| **Products** | Yog IDLE | `~/.yog/products/<id>/<version>` (a launcher entry for the OS) |
| **Build tools** | Rust toolchain and the five targets a `.yog` carries, zig, cargo-zigbuild, the Yog CLI | rustup's and cargo's own places (`~/.cargo`) |
| **Java** | JDK 17, 21, 25 (Temurin) | `~/.yog/jdk/<major>/<version>` |
| **SDKs** | Yog Mod Loader SDK (a release's source, run by its `build.sh`) | `~/.yog/sdk/loader/<version>` |
| **Game instances** (later) | Fabric 26.2 with the Yog jar of a release | `~/.yog/instances/<loader>-<mc>` |

Everything installed is listed in `~/.yog/components.ydata` (crate
`yog-home`, in Yog-Mod-Loader): id, version, path. Only the Installer
writes it. `yog run`, Yog IDLE and the Installer read it. A machine's own
overrides — our loader checkout, a JDK of its own, the NVIDIA card — go in
`~/.yog/local.toml`, written by hand and never touched by the Installer.

## The channel (catalog)

`catalog/catalog.toml` says what can be installed: products, workloads,
components, their groups, and where each one is published (`source`). It's
built into the Installer and refreshed from the latest Yog-Installer
release, so an Installer already out there learns of new components.
Versions aren't in it: they come live from where each source publishes
them — GitHub releases, crates.io, Adoptium, ziglang.org.

A source is one of:

| `kind` | Versions from | Installed by |
|---|---|---|
| `github` | the repo's releases; an asset per platform | download, unpack into the product's folder |
| `git` | the repo's release tags | the tag's source archive, unpacked |
| `crate` | crates.io | `cargo install --locked <name>@<version>` |
| `rustup` | the toolchain the catalog names | `rustup toolchain install` |
| `rust-targets` | the five `.yog` platforms | `rustup target add` |
| `adoptium` | Adoptium's API, newest of a major | download, unpack |
| `zig` | ziglang.org's index, newest stable | download, unpack |

## Compatibility

A release may carry `yog-release.json`: what it needs of the others
(`{"requires": {"yog.cli": ">=0.47.0"}}`). The Installer reads it with the
release, checks a selection against what's installed or chosen, and says
what doesn't fit before installing anything.

## Engine (`yog-installer-core`)

| Module | Job |
|---|---|
| `catalog` | the catalog's types, reading and checking it |
| `platform` | this machine's platform tag (`linux-x86_64`) |
| `http` | GET JSON, downloads with progress, verification, cancellation |
| `releases` | a source's versions |
| `state` | what's installed (components.ydata) and what's newer |
| `selection` | workloads + components chosen → components → changes |
| `compat` | version requirements |
| `install` | one change, per source kind |
| `engine` | a whole plan: download, install, record; progress events |

No file over 800 lines (in practice, under 300).

## App (`src-tauri/`, `src/`)

Visual Studio Installer, piece for piece:

- **Installed / Available** tabs of product cards: name and version, an
  *Update available* note with *Update*, *Modify*, *Launch*, *More* (Repair,
  Uninstall, Release notes), and the progress of a running operation
  (*Downloading and verifying*, *Installing: package N of M*, Pause, Cancel).
- **Modify** window: *Workloads* (cards with checkboxes), *Individual
  components* (search, groups, checkboxes), *Language packs*, *Installation
  locations*; on the right *Installation details* (each workload's included
  and optional parts); at the bottom the location, *Total space required*,
  *Install while downloading / Download all, then install*, and the
  Install/Modify button.
- Self-update first: when a newer Installer is out, it updates itself before
  anything else, like VS Installer.
