//! A Debian package unpacked without installing it (no root, no dpkg): its
//! `ar` container read for the `data.tar.*` member, which is unpacked as
//! any archive — the files it would have put under `/` land under a folder.

use super::archive::{self, Kind};
use crate::plans::progress::Cancel;
use std::io::{BufReader, Read};
use std::path::Path;

/// What an `ar` container starts with.
const AR_MAGIC: &[u8; 8] = b"!<arch>\n";
/// A member's header: its name, then dates and ids, then its size (decimal).
const HEADER: usize = 60;
const NAME: std::ops::Range<usize> = 0..16;
const SIZE: std::ops::Range<usize> = 48..58;
/// A member's name prefix holding the package's files.
const DATA: &str = "data.tar";

/// Unpacks the files of the package at `package` into `dest`.
pub(super) fn unpack(package: &Path, dest: &Path, cancel: &Cancel) -> Result<(), String> {
    let at = |e: std::io::Error| format!("{}: {e}", package.display());
    let mut reader = BufReader::new(std::fs::File::open(package).map_err(at)?);
    let mut magic = [0u8; AR_MAGIC.len()];
    reader.read_exact(&mut magic).map_err(at)?;
    if &magic != AR_MAGIC {
        return Err(format!("{}: not a Debian package", package.display()));
    }
    loop {
        let mut header = [0u8; HEADER];
        reader.read_exact(&mut header).map_err(at)?;
        let name = String::from_utf8_lossy(&header[NAME])
            .trim_end()
            .trim_end_matches('/')
            .to_string();
        let size: u64 = String::from_utf8_lossy(&header[SIZE])
            .trim()
            .parse()
            .map_err(|_| format!("{}: a member without a size", package.display()))?;
        if name.starts_with(DATA) {
            let kind = Kind::of(&name).ok_or_else(|| {
                format!(
                    "{}: its files are packed as {name}, which isn't read",
                    package.display()
                )
            })?;
            return archive::unpack_stream((&mut reader).take(size), kind, dest, false, cancel);
        }
        // Members are padded to an even length.
        let skip = size + size % 2;
        std::io::copy(&mut (&mut reader).take(skip), &mut std::io::sink()).map_err(at)?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An `ar` member: its header (name, zeroed fields, size, end mark), its data, padding.
    fn member(name: &str, data: &[u8]) -> Vec<u8> {
        let mut m = format!(
            "{name:<16}{:<12}{:<6}{:<6}{:<8}{:<10}`\n",
            0,
            0,
            0,
            644,
            data.len()
        )
        .into_bytes();
        m.extend_from_slice(data);
        if data.len() % 2 == 1 {
            m.push(b'\n');
        }
        m
    }

    fn data_tar_gz() -> Vec<u8> {
        let gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        let mut tar = tar::Builder::new(gz);
        let program = b"#!/bin/sh\n";
        let mut header = tar::Header::new_gnu();
        header.set_size(program.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        tar.append_data(&mut header, "./usr/bin/tool", &program[..])
            .unwrap();
        tar.into_inner().unwrap().finish().unwrap()
    }

    #[test]
    fn a_packages_files_land_under_the_folder() {
        let dir = std::env::temp_dir().join(format!("yog-installer-deb-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut deb = AR_MAGIC.to_vec();
        deb.extend(member("debian-binary", b"2.0\n"));
        deb.extend(member("control.tar.gz", b"odd"));
        deb.extend(member("data.tar.gz", &data_tar_gz()));
        let package = dir.join("p.deb");
        std::fs::write(&package, deb).unwrap();
        let out = dir.join("out");
        unpack(&package, &out, &Cancel::default()).unwrap();
        assert_eq!(
            std::fs::read(out.join("usr/bin/tool")).unwrap(),
            b"#!/bin/sh\n"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
