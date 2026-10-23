//! HTTPS: JSON and text from APIs (GitHub's cached, with its token when
//! there is one). Files are [`crate::sources::download`]'s.

use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::OnceLock;

/// How the Installer introduces itself (GitHub's API wants one).
const USER_AGENT: &str = concat!("yog-installer/", env!("CARGO_PKG_VERSION"));
/// GitHub's REST API, and the media type it answers in.
const GITHUB_API: &str = "https://api.github.com/";
const GITHUB_JSON: &str = "application/vnd.github+json";

/// GitHub's answers that mean "too many requests": a stale answer is better then.
const RATE_LIMITED: [u16; 2] = [403, 429];
const NOT_MODIFIED: u16 = 304;
/// Where answers of GitHub's API are kept (with their ETags), under the cache.
const HTTP_CACHE: &str = "http";

pub(crate) fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::Agent::config_builder()
            .user_agent(USER_AGENT)
            // Statuses are read, not raised: 304 and a rate limit are answers.
            .http_status_as_error(false)
            .build()
            .into()
    })
}

/// Where API answers are cached; set once by the app (`Home::cache_dir`).
static CACHE: OnceLock<PathBuf> = OnceLock::new();

/// Keeps API answers under `dir`, so a repeated question costs nothing (a
/// `304` isn't counted against GitHub's limit) and a rate limit isn't an error.
pub fn set_cache(dir: PathBuf) {
    let _ = CACHE.set(dir.join(HTTP_CACHE));
}

/// A GitHub token, when this machine has one: `GITHUB_TOKEN`, else the GitHub
/// CLI's (`gh auth token`). It lifts the limit from 60 requests an hour to 5000.
fn github_token() -> Option<&'static str> {
    static TOKEN: OnceLock<Option<String>> = OnceLock::new();
    TOKEN
        .get_or_init(|| {
            std::env::var("GITHUB_TOKEN")
                .ok()
                .filter(|t| !t.is_empty())
                .or_else(|| {
                    let out =
                        substrate_platform::process::command(crate::machine::platform::exe("gh"))
                            .args(["auth", "token"])
                            .output()
                            .ok()?;
                    let token = String::from_utf8(out.stdout).ok()?.trim().to_string();
                    (out.status.success() && !token.is_empty()).then_some(token)
                })
        })
        .as_deref()
}

/// A cached answer: its ETag and its body.
#[derive(yog_data::Schema)]
struct Cached {
    etag: Option<String>,
    body: String,
}

fn cache_file(url: &str) -> Option<PathBuf> {
    Some(CACHE.get()?.join(format!(
        "{}{}{}",
        hex(&Sha256::digest(url.as_bytes())),
        yog_ids::sep::DOT,
        yog_ids::Format::Data.ext()
    )))
}

/// `url`'s JSON.
pub fn get_json<T: DeserializeOwned>(url: &str) -> Result<T, String> {
    let text = if url.starts_with(GITHUB_API) {
        github(url)?
    } else {
        get_text(url)?
    };
    serde_json::from_str(&text).map_err(|e| format!("{url}: {e}"))
}

/// A GitHub API answer: asked again only if it changed, the cached one when rate-limited.
fn github(url: &str) -> Result<String, String> {
    let file = cache_file(url);
    let cached: Option<Cached> = file
        .as_ref()
        .and_then(|f| std::fs::read(f).ok())
        .and_then(|b| yog_data::read_bytes(&b).ok());
    let mut req = agent().get(url).header("Accept", GITHUB_JSON);
    if let Some(token) = github_token() {
        req = req.header("Authorization", &format!("Bearer {token}"));
    }
    if let Some(etag) = cached.as_ref().and_then(|c| c.etag.as_deref()) {
        req = req.header("If-None-Match", etag);
    }
    let mut resp = req.call().map_err(|e| format!("{url}: {e}"))?;
    let status = resp.status().as_u16();
    match cached {
        Some(c) if status == NOT_MODIFIED || RATE_LIMITED.contains(&status) => return Ok(c.body),
        _ => {}
    }
    if !resp.status().is_success() {
        return Err(format!("{url}: {}", resp.status()));
    }
    let etag = resp
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let body = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("{url}: {e}"))?;
    if let Some(file) = file {
        let _ = std::fs::create_dir_all(file.parent().unwrap_or(&file));
        let cached = Cached {
            etag,
            body: body.clone(),
        };
        let _ = std::fs::write(file, yog_data::to_bytes(&cached));
    }
    Ok(body)
}

/// A response that has to have succeeded.
pub(crate) fn ok(
    url: &str,
    resp: ureq::http::Response<ureq::Body>,
) -> Result<ureq::http::Response<ureq::Body>, String> {
    if resp.status().is_success() {
        Ok(resp)
    } else {
        Err(format!("{url}: {}", resp.status()))
    }
}

/// `url`'s bytes (a small file: an icon).
pub fn get_bytes(url: &str) -> Result<Vec<u8>, String> {
    let resp = ok(
        url,
        agent().get(url).call().map_err(|e| format!("{url}: {e}"))?,
    )?;
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut resp.into_body().into_reader(), &mut bytes)
        .map_err(|e| format!("{url}: {e}"))?;
    Ok(bytes)
}

/// `url`'s text.
pub fn get_text(url: &str) -> Result<String, String> {
    let mut resp = ok(
        url,
        agent().get(url).call().map_err(|e| format!("{url}: {e}"))?,
    )?;
    resp.body_mut()
        .read_to_string()
        .map_err(|e| format!("{url}: {e}"))
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
