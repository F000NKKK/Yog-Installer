//! This machine's platform, in the words Yog's releases use.

/// An executable's file name here (`.exe` on Windows).
pub fn exe(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

/// What lists the system's shared libraries (glibc's).
const LDCONFIG: [&str; 2] = ["/sbin/ldconfig", "/usr/sbin/ldconfig"];
const LIST_LIBRARIES: &str = "-p";

/// Whether the system has every shared library in `sonames` (Linux; none
/// elsewhere, nor without `ldconfig`).
pub fn has_libraries(sonames: &[String]) -> bool {
    if !cfg!(target_os = "linux") {
        return false;
    }
    let Some(list) = LDCONFIG.iter().find_map(|ldconfig| {
        let out = std::process::Command::new(ldconfig)
            .arg(LIST_LIBRARIES)
            .output()
            .ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    }) else {
        return false;
    };
    // Each line: `\tlibfoo.so.1 (libc6,x86-64) => /path`.
    sonames.iter().all(|so| {
        list.lines()
            .any(|l| l.split_whitespace().next() == Some(so.as_str()))
    })
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
