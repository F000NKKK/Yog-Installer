//! This machine's platform, in the words Yog's releases use.

/// `linux-x86_64`, `windows-x86_64`, `macos-aarch64`, … — the tag Yog's
/// natives and the catalog's per-platform assets are keyed by.
pub fn tag() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

/// An executable's file name here (`.exe` on Windows).
pub fn exe(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

/// Adoptium's names for this OS and architecture.
pub fn adoptium() -> Option<(&'static str, &'static str)> {
    let os = match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "mac",
        "windows" => "windows",
        _ => return None,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "aarch64",
        _ => return None,
    };
    Some((os, arch))
}

/// ziglang.org's key for this host: `x86_64-linux`.
pub fn zig() -> String {
    format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS)
}

/// rustup's host triple here (rustup-init's download), none where Rust has no host.
pub fn rustup_host() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Some("x86_64-unknown-linux-gnu"),
        ("linux", "aarch64") => Some("aarch64-unknown-linux-gnu"),
        ("windows", "x86_64") => Some("x86_64-pc-windows-msvc"),
        ("macos", "x86_64") => Some("x86_64-apple-darwin"),
        ("macos", "aarch64") => Some("aarch64-apple-darwin"),
        _ => None,
    }
}

/// What an AppImage's runtime tells the program inside about itself.
const APPIMAGE_VARS: [&str; 4] = ["APPDIR", "APPIMAGE", "ARGV0", "OWD"];

/// A command for `program` with the system's own environment: nothing an
/// AppImage set up for the Installer inside it (`LD_LIBRARY_PATH` into its
/// mount, its GTK and GIO modules, ...) — a program started with those loads
/// the Installer's libraries instead of its own, and fails.
pub fn command(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    let mut cmd = std::process::Command::new(program);
    if let Some(appdir) = std::env::var_os("APPDIR").filter(|d| !d.is_empty()) {
        let appdir = appdir.to_string_lossy().into_owned();
        for (key, value) in std::env::vars_os() {
            if value.to_string_lossy().contains(&appdir) {
                cmd.env_remove(key);
            }
        }
        for key in APPIMAGE_VARS {
            cmd.env_remove(key);
        }
    }
    cmd
}
