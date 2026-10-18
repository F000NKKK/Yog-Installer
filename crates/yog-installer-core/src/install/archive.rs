//! Unpacking downloads: zip, tar.gz, tar.xz — never outside the folder
//! they go into, optionally without the one folder they're all in.

use crate::progress::Cancel;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

/// How an archive is packed, by its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Zip,
    TarGz,
    TarXz,
}

impl Kind {
    /// The kind a file named `name` is; none when it isn't an archive.
    pub fn of(name: &str) -> Option<Self> {
        let n = name.to_ascii_lowercase();
        if n.ends_with(".zip") {
            Some(Self::Zip)
        } else if n.ends_with(".tar.gz") || n.ends_with(".tgz") {
            Some(Self::TarGz)
        } else if n.ends_with(".tar.xz") {
            Some(Self::TarXz)
        } else {
            None
        }
    }
}

/// Unpacks `archive` into `dest`; `strip`: without the top folder every entry is in.
pub fn unpack(
    archive: &Path,
    kind: Kind,
    dest: &Path,
    strip: bool,
    cancel: &Cancel,
) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|e| format!("{}: {e}", dest.display()))?;
    let file = std::fs::File::open(archive).map_err(|e| format!("{}: {e}", archive.display()))?;
    let at = |e: std::io::Error| format!("{}: {e}", archive.display());
    match kind {
        Kind::Zip => {
            unzip(file, dest, strip, cancel).map_err(|e| format!("{}: {e}", archive.display()))
        }
        Kind::TarGz => untar(flate2::read::GzDecoder::new(file), dest, strip, cancel).map_err(at),
        Kind::TarXz => untar(xz2::read::XzDecoder::new(file), dest, strip, cancel).map_err(at),
    }
}

/// `path` inside `dest`, a top folder dropped if `strip`; none for what would land outside.
fn inside(dest: &Path, path: &Path, strip: bool) -> Option<PathBuf> {
    let parts: Vec<Component> = path.components().collect();
    if parts
        .iter()
        .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
    {
        return None;
    }
    let rest: PathBuf = parts.into_iter().skip(usize::from(strip)).collect();
    (!rest.as_os_str().is_empty()).then(|| dest.join(rest))
}

fn untar(reader: impl Read, dest: &Path, strip: bool, cancel: &Cancel) -> std::io::Result<()> {
    let mut tar = tar::Archive::new(reader);
    tar.set_preserve_permissions(true);
    for entry in tar.entries()? {
        cancel.check().map_err(std::io::Error::other)?;
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let Some(to) = inside(dest, &path, strip) else {
            continue;
        };
        if let Some(dir) = to.parent() {
            std::fs::create_dir_all(dir)?;
        }
        entry.unpack(&to)?;
    }
    Ok(())
}

fn unzip(file: std::fs::File, dest: &Path, strip: bool, cancel: &Cancel) -> Result<(), String> {
    let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    for i in 0..zip.len() {
        cancel.check()?;
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let Some(to) = entry.enclosed_name().and_then(|p| inside(dest, &p, strip)) else {
            continue;
        };
        if entry.is_dir() {
            std::fs::create_dir_all(&to).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(dir) = to.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let mut out = std::fs::File::create(&to).map_err(|e| format!("{}: {e}", to.display()))?;
        std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&to, std::fs::Permissions::from_mode(mode))
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_by_name() {
        assert_eq!(
            Kind::of("OpenJDK21U-jdk_x64_linux.tar.gz"),
            Some(Kind::TarGz)
        );
        assert_eq!(
            Kind::of("zig-x86_64-linux-0.13.0.tar.xz"),
            Some(Kind::TarXz)
        );
        assert_eq!(Kind::of("jdk.ZIP"), Some(Kind::Zip));
        assert_eq!(Kind::of("Yog.IDLE_2.0.0_amd64.AppImage"), None);
    }

    #[test]
    fn nothing_lands_outside() {
        let dest = Path::new("/d");
        assert_eq!(
            inside(dest, Path::new("top/bin/java"), true),
            Some(PathBuf::from("/d/bin/java"))
        );
        assert_eq!(inside(dest, Path::new("top/"), true), None);
        assert_eq!(inside(dest, Path::new("../etc/passwd"), false), None);
        assert_eq!(inside(dest, Path::new("/etc/passwd"), false), None);
    }

    #[test]
    fn a_tar_gz_unpacks_without_its_top_folder() {
        let dir =
            std::env::temp_dir().join(format!("yog-installer-archive-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let archive = dir.join("a.tar.gz");
        {
            let gz = flate2::write::GzEncoder::new(
                std::fs::File::create(&archive).unwrap(),
                flate2::Compression::fast(),
            );
            let mut tar = tar::Builder::new(gz);
            let data = b"hi";
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            tar.append_data(&mut header, "top/bin/tool", &data[..])
                .unwrap();
            tar.into_inner().unwrap().finish().unwrap();
        }
        unpack(
            &archive,
            Kind::TarGz,
            &dir.join("out"),
            true,
            &Cancel::default(),
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("out/bin/tool")).unwrap(),
            "hi"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
