//! HTTPS: JSON from APIs, files with progress, verified and cancellable.

use crate::progress::Cancel;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::OnceLock;

/// How the Installer introduces itself (GitHub's API wants one).
const USER_AGENT: &str = concat!("yog-installer/", env!("CARGO_PKG_VERSION"));
/// HTTP's answer to a `Range` request it honors.
const PARTIAL_CONTENT: u16 = 206;
/// Bytes read between progress reports and cancel checks.
const CHUNK: usize = 64 * 1024;
/// GitHub's REST API, and the media type it answers in.
const GITHUB_API: &str = "https://api.github.com/";
const GITHUB_JSON: &str = "application/vnd.github+json";

fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::Agent::config_builder()
            .user_agent(USER_AGENT)
            .build()
            .into()
    })
}

/// `url`'s JSON.
pub fn get_json<T: DeserializeOwned>(url: &str) -> Result<T, String> {
    let mut req = agent().get(url);
    if url.starts_with(GITHUB_API) {
        req = req.header("Accept", GITHUB_JSON);
        // A token lifts GitHub's limit of 60 requests an hour, when there is one.
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            req = req.header("Authorization", &format!("Bearer {token}"));
        }
    }
    let mut resp = req.call().map_err(|e| format!("{url}: {e}"))?;
    resp.body_mut()
        .read_json()
        .map_err(|e| format!("{url}: {e}"))
}

/// `url`'s text.
pub fn get_text(url: &str) -> Result<String, String> {
    let mut resp = agent().get(url).call().map_err(|e| format!("{url}: {e}"))?;
    resp.body_mut()
        .read_to_string()
        .map_err(|e| format!("{url}: {e}"))
}

/// Downloads `url` to `dest`, telling `progress(done, total)` as it goes; when
/// `sha256` is given (hex), the file must have it. A partial file is removed.
pub fn download(
    url: &str,
    dest: &Path,
    sha256: Option<&str>,
    cancel: &Cancel,
    progress: &dyn Fn(u64, Option<u64>),
) -> Result<(), String> {
    let result = fetch(url, dest, sha256, cancel, progress);
    if result.is_err() {
        let _ = std::fs::remove_file(dest);
    }
    result
}

fn fetch(
    url: &str,
    dest: &Path,
    sha256: Option<&str>,
    cancel: &Cancel,
    progress: &dyn Fn(u64, Option<u64>),
) -> Result<(), String> {
    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let at = |e: std::io::Error| format!("{}: {e}", dest.display());
    let mut file = std::fs::File::create(dest).map_err(at)?;
    let mut hash = Sha256::new();
    let mut buf = vec![0u8; CHUNK];
    let (mut done, mut total) = (0u64, None);
    // One connection per stretch between pauses: a pause closes it, so the
    // download really stops; going on asks for the rest (`Range`).
    loop {
        cancel.check()?;
        let mut req = agent().get(url);
        if done > 0 {
            req = req.header("Range", &format!("bytes={done}-"));
        }
        let resp = req.call().map_err(|e| format!("{url}: {e}"))?;
        if done > 0 && resp.status().as_u16() != PARTIAL_CONTENT {
            // The server can't send the rest: the whole file again.
            file.set_len(0).map_err(at)?;
            file.seek(SeekFrom::Start(0)).map_err(at)?;
            (hash, done) = (Sha256::new(), 0);
        }
        let length: Option<u64> = resp
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok());
        total = total.or(length.map(|l| l + done));
        let mut body = resp.into_body().into_reader();
        loop {
            if cancel.is_paused() {
                break;
            }
            cancel.check()?;
            let n = body.read(&mut buf).map_err(|e| format!("{url}: {e}"))?;
            if n == 0 {
                return verify(url, hash, sha256);
            }
            file.write_all(&buf[..n]).map_err(at)?;
            hash.update(&buf[..n]);
            done += n as u64;
            progress(done, total);
        }
    }
}

/// The finished download's hash against the one it must have.
fn verify(url: &str, hash: Sha256, sha256: Option<&str>) -> Result<(), String> {
    let Some(want) = sha256 else { return Ok(()) };
    let got = hex(&hash.finalize());
    if got.eq_ignore_ascii_case(want) {
        Ok(())
    } else {
        Err(format!(
            "{url}: the download is damaged (sha256 {got}, not {want})"
        ))
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
