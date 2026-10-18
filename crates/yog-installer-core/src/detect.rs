//! What's installed: what the Installer put there (`components.json`), and
//! what was there before it — rustup's toolchains, `cargo install`ed tools,
//! the system's JDKs, a zig on `PATH`. Only reads.

use crate::catalog::{Catalog, Source};
use crate::platform;
use serde::Serialize;
use std::path::{Path, PathBuf};
use yog_home::components::Components;

/// An installed component.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    pub id: String,
    pub version: String,
    pub path: Option<PathBuf>,
    /// Installed by the Installer (it may update and remove it), not found on the system.
    pub managed: bool,
}

/// The machine, as detection asks it (a fake one in tests).
pub trait System {
    /// `program args`' standard output when it runs and succeeds.
    fn run(&self, program: &Path, args: &[&str]) -> Option<String>;
    fn read(&self, path: &Path) -> Option<String>;
    fn list_dir(&self, path: &Path) -> Vec<PathBuf>;
    fn env(&self, key: &str) -> Option<String>;
    fn home(&self) -> Option<PathBuf>;
}

/// The real machine.
pub struct RealSystem;

impl System for RealSystem {
    fn run(&self, program: &Path, args: &[&str]) -> Option<String> {
        let out = std::process::Command::new(program)
            .args(args)
            .output()
            .ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    }

    fn read(&self, path: &Path) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }

    fn list_dir(&self, path: &Path) -> Vec<PathBuf> {
        std::fs::read_dir(path)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .collect()
    }

    fn env(&self, key: &str) -> Option<String> {
        std::env::var(key).ok().filter(|v| !v.is_empty())
    }

    fn home(&self) -> Option<PathBuf> {
        dirs::home_dir()
    }
}

/// Where JDKs are kept by the OS's and the usual installers.
const JDK_FOLDERS: &[&str] = &[
    "/usr/lib/jvm",
    "/Library/Java/JavaVirtualMachines",
    "C:\\Program Files\\Eclipse Adoptium",
    "C:\\Program Files\\Java",
];
/// A JDK's `release` file line that says its version.
const JAVA_VERSION: &str = "JAVA_VERSION=";

/// Every catalog component installed, by the Installer or before it.
pub fn installed(catalog: &Catalog, managed: &Components, system: &dyn System) -> Vec<Found> {
    let mut out: Vec<Found> = managed
        .installed
        .iter()
        .map(|c| Found {
            id: c.id.clone(),
            version: c.version.clone(),
            path: Some(c.path.clone()),
            managed: true,
        })
        .collect();
    for c in &catalog.components {
        if out.iter().any(|f| f.id == c.id) {
            continue;
        }
        if let Some((version, path)) = on_system(&c.source, system) {
            out.push(Found {
                id: c.id.clone(),
                version,
                path,
                managed: false,
            });
        }
    }
    out
}

/// A component of `source` the system has, and where.
fn on_system(source: &Source, system: &dyn System) -> Option<(String, Option<PathBuf>)> {
    let bin = cargo_bin(system);
    match source {
        Source::Rustup { toolchain } => {
            let list = system.run(&bin.join(platform::exe("rustup")), &["toolchain", "list"])?;
            list.lines()
                .any(|l| l.starts_with(&format!("{toolchain}-")))
                .then(|| (toolchain.clone(), None))
        }
        Source::RustTargets { toolchain, targets } => {
            let list = system.run(
                &bin.join(platform::exe("rustup")),
                &["target", "list", "--installed", "--toolchain", toolchain],
            )?;
            targets
                .iter()
                .all(|t| list.lines().any(|l| l.trim() == t))
                .then(|| (toolchain.clone(), None))
        }
        Source::Crate { name } => {
            let list = system.run(&bin.join(platform::exe("cargo")), &["install", "--list"])?;
            crate_version(&list, name).map(|v| (v, Some(bin.join(platform::exe(name)))))
        }
        Source::Adoptium { major } => system_jdk(*major, system).map(|(v, p)| (v, Some(p))),
        Source::Zig => {
            let v = system.run(Path::new(&platform::exe("zig")), &["version"])?;
            Some((v.trim().to_string(), None))
        }
        Source::Github { .. } | Source::Git { .. } => None,
    }
}

/// cargo's and rustup's folder: `$CARGO_HOME/bin`, else `~/.cargo/bin`.
pub fn cargo_bin(system: &dyn System) -> PathBuf {
    let home = system
        .env("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| system.home().map(|h| h.join(".cargo")));
    home.unwrap_or_default().join("bin")
}

/// `name`'s version in `cargo install --list` (`yog-cli v0.47.1:`).
fn crate_version(list: &str, name: &str) -> Option<String> {
    list.lines().find_map(|l| {
        l.strip_prefix(&format!("{name} v"))?
            .split([':', ' '])
            .next()
            .map(str::to_string)
    })
}

/// The newest JDK of `major` the system has: `JAVA_HOME`, then the usual folders.
fn system_jdk(major: u32, system: &dyn System) -> Option<(String, PathBuf)> {
    let homes = system.env("JAVA_HOME").map(PathBuf::from).into_iter();
    let homes = homes.chain(
        JDK_FOLDERS
            .iter()
            .flat_map(|f| system.list_dir(Path::new(f))),
    );
    homes
        .flat_map(|h| [h.join("Contents/Home"), h])
        .filter_map(|h| Some((jdk_version(&system.read(&h.join("release"))?)?, h)))
        .filter(|(v, _)| v.split('.').next().and_then(|m| m.parse().ok()) == Some(major))
        .max_by(|a, b| crate::compat::compare(&a.0, &b.0))
}

/// The version in a JDK's `release` file (`JAVA_VERSION="21.0.5"`).
fn jdk_version(release: &str) -> Option<String> {
    let line = release.lines().find_map(|l| l.strip_prefix(JAVA_VERSION))?;
    Some(line.trim().trim_matches('"').to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Fake {
        runs: HashMap<String, String>,
        files: HashMap<PathBuf, String>,
        dirs: HashMap<PathBuf, Vec<PathBuf>>,
    }

    impl System for Fake {
        fn run(&self, program: &Path, args: &[&str]) -> Option<String> {
            let name = program.file_name()?.to_string_lossy();
            self.runs
                .get(&format!("{name} {}", args.join(" ")))
                .cloned()
        }
        fn read(&self, path: &Path) -> Option<String> {
            self.files.get(path).cloned()
        }
        fn list_dir(&self, path: &Path) -> Vec<PathBuf> {
            self.dirs.get(path).cloned().unwrap_or_default()
        }
        fn env(&self, _: &str) -> Option<String> {
            None
        }
        fn home(&self) -> Option<PathBuf> {
            Some(PathBuf::from("/home/u"))
        }
    }

    #[test]
    fn finds_what_was_there_before() {
        let mut sys = Fake::default();
        sys.runs.insert(
            format!("{} toolchain list", platform::exe("rustup")),
            "1.99-x86_64-unknown-linux-gnu (default)\n".into(),
        );
        sys.runs.insert(
            format!("{} install --list", platform::exe("cargo")),
            "cargo-zigbuild v0.20.1:\n    cargo-zigbuild\nyog-cli v0.47.1:\n    yog\n".into(),
        );
        sys.dirs.insert(
            "/usr/lib/jvm".into(),
            vec!["/usr/lib/jvm/java-21".into(), "/usr/lib/jvm/java-17".into()],
        );
        sys.files.insert(
            "/usr/lib/jvm/java-21/release".into(),
            "JAVA_VERSION=\"21.0.5\"\n".into(),
        );
        sys.files.insert(
            "/usr/lib/jvm/java-17/release".into(),
            "JAVA_VERSION=\"17.0.13\"\n".into(),
        );
        let found = installed(&Catalog::built_in(), &Components::default(), &sys);
        let version = |id: &str| {
            found
                .iter()
                .find(|f| f.id == id)
                .map(|f| f.version.as_str())
        };
        assert_eq!(version("rust.toolchain"), Some("1.99"));
        assert_eq!(version("yog.cli"), Some("0.47.1"));
        assert_eq!(version("cargo-zigbuild"), Some("0.20.1"));
        assert_eq!(version("yog.jdk.21"), Some("21.0.5"));
        assert_eq!(version("yog.jdk.25"), None);
        assert!(found.iter().all(|f| !f.managed));
    }
}
